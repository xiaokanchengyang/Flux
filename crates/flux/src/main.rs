use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod interactive;
mod extract;
mod tui;

#[cfg(feature = "cloud")]
mod cloud_handler;

#[derive(Parser)]
#[command(name = "flux")]
#[command(author, version, about = "A cross-platform file archiver and compressor", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Show progress bar
    #[arg(long, global = true)]
    progress: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract files from an archive
    Extract {
        /// Archive file to extract
        archive: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,

        /// Skip existing files
        #[arg(long, conflicts_with = "overwrite")]
        skip: bool,

        /// Rename files if they exist
        #[arg(long, conflicts_with_all = ["overwrite", "skip"])]
        rename: bool,

        /// Remove the specified number of leading path elements
        #[arg(long)]
        strip_components: Option<usize>,
        
        /// Enable interactive mode for conflict resolution
        #[arg(long, short = 'i', conflicts_with_all = ["overwrite", "skip", "rename"])]
        interactive: bool,

        /// If the archive contains a single folder, hoist its contents to the output directory
        #[arg(long, help = "If the archive contains a single folder, hoist its contents to the output directory")]
        hoist: bool,
    },

    /// Pack files into an archive
    Pack {
        /// Input file or directory
        input: PathBuf,

        /// Output archive file
        #[arg(short, long)]
        output: PathBuf,

        /// Archive format (zip, tar, tar.gz, tar.zst, tar.xz)
        #[arg(short, long)]
        format: Option<String>,

        /// Enable smart compression strategy
        #[arg(long)]
        smart: bool,

        /// Compression algorithm (zstd, xz, brotli, gzip)
        #[arg(long, conflicts_with = "smart")]
        algo: Option<String>,

        /// Compression level (1-9 for most algorithms)
        #[arg(long)]
        level: Option<u32>,

        /// Number of threads to use
        #[arg(long)]
        threads: Option<usize>,

        /// Follow symlinks (pack link targets instead of links)
        #[arg(long)]
        follow_symlinks: bool,

        /// Force compression on already compressed files
        #[arg(long)]
        force_compress: bool,
        
        /// Previous manifest file for incremental backup
        #[arg(long)]
        incremental: Option<PathBuf>,
    },

    /// Inspect archive contents
    Inspect {
        /// Archive file to inspect
        archive: PathBuf,

        /// Output format as JSON
        #[arg(long)]
        json: bool,
        
        /// Interactive TUI mode
        #[arg(short, long)]
        interactive: bool,
        
        /// Show as tree structure
        #[arg(long)]
        tree: bool,
    },

    /// Show or edit configuration
    Config {
        /// Show current configuration
        #[arg(long, conflicts_with_all = ["edit", "path"])]
        show: bool,

        /// Edit configuration file
        #[arg(long, conflicts_with_all = ["show", "path"])]
        edit: bool,

        /// Show configuration file path
        #[arg(long, conflicts_with_all = ["show", "edit"])]
        path: bool,
    },
    
    /// Synchronize directory with incremental backup
    Sync {
        /// Source directory to backup
        source: PathBuf,
        
        /// Target archive file
        target: PathBuf,
        
        /// Compression algorithm (zstd, xz, brotli, gzip)
        #[arg(long)]
        algo: Option<String>,
        
        /// Compression level (1-9 for most algorithms)
        #[arg(long)]
        level: Option<u32>,
        
        /// Number of threads to use
        #[arg(long)]
        threads: Option<usize>,
        
        /// Follow symlinks (pack link targets instead of links)
        #[arg(long)]
        follow_symlinks: bool,
        
        /// Force full backup (ignore previous manifest)
        #[arg(long)]
        full: bool,
    },
}

fn setup_logging(verbose: bool, quiet: bool) {
    if quiet {
        return;
    }

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(std::io::stderr)
        .init();
}

fn main() {
    let result = run();

    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            error!("Error: {}", e);

            // Map errors to exit codes based on requirements
            let exit_code = map_error_to_exit_code(&e);
            process::exit(exit_code);
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, cli.quiet);

    match cli.command {
        Commands::Extract {
            archive,
            output,
            overwrite,
            skip,
            rename,
            strip_components,
            interactive,
            hoist,
        } => {
            let archive_str = archive.to_string_lossy();
            info!("Extracting archive: {}", archive_str);
            let output_dir = output.unwrap_or_else(|| PathBuf::from("."));

            // Check if the archive is a cloud path
            #[cfg(feature = "cloud")]
            if cloud_handler::is_cloud_path(&archive_str) {
                info!("Detected cloud archive: {}", cloud_handler::describe_cloud_location(&archive_str));
                
                // Check credentials
                cloud_handler::check_cloud_credentials(&archive_str)?;
                
                // Create cloud reader
                let mut reader = cloud_handler::create_cloud_reader(&archive_str)?;
                
                // Create a temporary file to store the archive
                let temp_dir = tempfile::tempdir()?;
                let temp_archive = temp_dir.path().join("cloud_archive.tar");
                let mut temp_file = std::fs::File::create(&temp_archive)?;
                
                // Download the archive to temp file
                info!("Downloading archive from cloud storage...");
                std::io::copy(&mut reader, &mut temp_file)?;
                drop(temp_file);
                
                // Extract from the temporary file
                if interactive {
                    info!("Interactive mode enabled - prompting for file conflicts");
                    extract::extract_interactive(&temp_archive, &output_dir, strip_components, cli.progress, hoist)?;
                } else {
                    let options = flux_core::archive::ExtractOptions {
                        overwrite,
                        skip,
                        rename,
                        strip_components,
                        hoist,
                    };

                    flux_core::archive::extract_with_options(&temp_archive, &output_dir, options)?;
                }
                
                info!("Extraction complete");
                return Ok(());
            }

            // Regular local file extraction
            if interactive {
                info!("Interactive mode enabled - prompting for file conflicts");
                extract::extract_interactive(&archive, &output_dir, strip_components, cli.progress, hoist)?;
            } else {
                let options = flux_core::archive::ExtractOptions {
                    overwrite,
                    skip,
                    rename,
                    strip_components,
                    hoist,
                };

                flux_core::archive::extract_with_options(&archive, &output_dir, options)?;
                info!("Extraction complete");
            }
        }

        Commands::Pack {
            input,
            output,
            format,
            smart,
            algo,
            level,
            threads,
            follow_symlinks,
            force_compress,
            incremental,
        } => {
            let output_str = output.to_string_lossy();
            info!("Packing {:?} into {}", input, output_str);

            // Warn about XZ thread limitations
            if let Some(ref algorithm) = algo {
                if algorithm.to_lowercase() == "xz" && threads.unwrap_or(2) > 1 {
                    info!("Note: XZ compression will be limited to single thread for stability");
                }
            }

            // Check if output is a cloud path
            #[cfg(feature = "cloud")]
            if cloud_handler::is_cloud_path(&output_str) {
                info!("Detected cloud output: {}", cloud_handler::describe_cloud_location(&output_str));
                
                // Check credentials
                cloud_handler::check_cloud_credentials(&output_str)?;
                
                if incremental.is_some() {
                    error!("Incremental backup to cloud storage is not yet supported");
                    return Err(anyhow::anyhow!("Incremental backup to cloud storage is not yet supported"));
                }
                
                // Create a temporary file for the archive
                let temp_dir = tempfile::tempdir()?;
                let temp_archive = temp_dir.path().join("temp_archive.tar");
                
                // Pack to temporary file
                let options = flux_core::archive::PackOptions {
                    smart,
                    algorithm: algo,
                    level,
                    threads,
                    force_compress,
                    follow_symlinks,
                };

                flux_core::archive::pack_with_strategy(&input, &temp_archive, format.as_deref(), options)?;
                
                // Upload to cloud
                info!("Uploading archive to cloud storage...");
                let mut cloud_writer = cloud_handler::create_cloud_writer(&output_str)?;
                let mut temp_file = std::fs::File::open(&temp_archive)?;
                std::io::copy(&mut temp_file, &mut cloud_writer)?;
                cloud_writer.flush()?;
                
                info!("Packing complete - archive uploaded to cloud");
                return Ok(());
            }

            // Regular local file packing
            if let Some(manifest_path) = incremental {
                // Incremental backup mode
                info!("Performing incremental backup using manifest: {:?}", manifest_path);
                
                if !input.is_dir() {
                    error!("Incremental backup requires a directory as input");
                    return Err(anyhow::anyhow!("Incremental backup requires a directory as input"));
                }
                
                let (new_manifest_path, diff) = flux_core::archive::incremental::pack_incremental(
                    &input,
                    &output,
                    &manifest_path,
                    flux_core::archive::PackOptions {
                        smart,
                        algorithm: algo,
                        level,
                        threads,
                        force_compress,
                        follow_symlinks,
                    },
                )?;
                
                info!("Incremental backup complete");
                info!("Changes: {} added, {} modified, {} deleted", 
                    diff.added.len(), diff.modified.len(), diff.deleted.len());
                info!("New manifest saved to: {:?}", new_manifest_path);
            } else {
                // Regular packing mode
                let options = flux_core::archive::PackOptions {
                    smart,
                    algorithm: algo,
                    level,
                    threads,
                    force_compress,
                    follow_symlinks,
                };

                flux_core::archive::pack_with_strategy(&input, &output, format.as_deref(), options)?;
                
                // Generate manifest for future incremental backups
                if input.is_dir() {
                    let manifest = flux_core::manifest::Manifest::from_directory(&input)?;
                    let manifest_path = output.with_extension("manifest.json");
                    manifest.save(&manifest_path)?;
                    info!("Manifest saved to: {:?} (use with --incremental for future backups)", manifest_path);
                }
                
                info!("Packing complete");
            }
        }

        Commands::Inspect { archive, json, interactive, tree } => {
            let archive_str = archive.to_string_lossy();
            info!("Inspecting archive: {}", archive_str);

            let entries = {
                #[cfg(feature = "cloud")]
                {
                    if cloud_handler::is_cloud_path(&archive_str) {
                        info!("Detected cloud archive: {}", cloud_handler::describe_cloud_location(&archive_str));
                        
                        // Check credentials
                        cloud_handler::check_cloud_credentials(&archive_str)?;
                        
                        // Create cloud reader
                        let mut reader = cloud_handler::create_cloud_reader(&archive_str)?;
                        
                        // Create a temporary file to store the archive
                        let temp_dir = tempfile::tempdir()?;
                        let temp_archive = temp_dir.path().join("cloud_archive.tar");
                        let mut temp_file = std::fs::File::create(&temp_archive)?;
                        
                        // Download the archive to temp file
                        info!("Downloading archive from cloud storage...");
                        std::io::copy(&mut reader, &mut temp_file)?;
                        drop(temp_file);
                        
                        // Inspect the temporary file
                        flux_core::inspect(&temp_archive)?
                    } else {
                        flux_core::inspect(&archive)?
                    }
                }
                
                #[cfg(not(feature = "cloud"))]
                flux_core::inspect(&archive)?
            };

            if interactive {
                // Interactive TUI mode
                info!("Launching interactive browser...");
                tui::run_tui(entries)?;
            } else if json {
                // Output as JSON
                let json_output = serde_json::to_string_pretty(&entries)?;
                println!("{}", json_output);
            } else if tree {
                // Tree view
                print_tree(&entries);
            } else {
                // Output as human-readable table
                println!(
                    "{:<50} {:>15} {:>15} {:>10} {:>20}",
                    "Path", "Size", "Compressed", "Mode", "Modified"
                );
                println!("{}", "-".repeat(120));

                for entry in entries {
                    let mode_str = if let Some(mode) = entry.mode {
                        format!("{:o}", mode)
                    } else {
                        "-".to_string()
                    };

                    let mtime_str = if let Some(mtime) = entry.mtime {
                        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(mtime, 0)
                            .unwrap_or_default();
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    } else {
                        "-".to_string()
                    };

                    let compressed_str = entry
                        .compressed_size
                        .map(|s| format!("{}", s))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<50} {:>15} {:>15} {:>10} {:>20}",
                        entry.path.display(),
                        entry.size,
                        compressed_str,
                        mode_str,
                        mtime_str
                    );
                }
            }

            info!("Inspection complete");
        }

        Commands::Config { show, edit, path } => {
            use flux_core::config::Config;

            if show {
                // Show current configuration
                match Config::load() {
                    Ok(config) => {
                        let toml_str = toml::to_string_pretty(&config)?;
                        println!("{}", toml_str);
                    }
                    Err(e) => {
                        error!("Failed to load configuration: {}", e);
                        return Err(e.into());
                    }
                }
            } else if edit {
                // Open configuration file in editor
                let config_path = Config::config_path()
                    .map_err(|e| anyhow::anyhow!("Failed to get config path: {}", e))?;

                // Ensure config exists
                if !config_path.exists() {
                    info!("Creating default configuration file...");
                    Config::default()
                        .save()
                        .map_err(|e| anyhow::anyhow!("Failed to save default config: {}", e))?;
                }

                // Open in default editor
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });

                info!("Opening configuration file in {}", editor);
                std::process::Command::new(&editor)
                    .arg(&config_path)
                    .status()
                    .map_err(|e| anyhow::anyhow!("Failed to open editor: {}", e))?;
            } else if path {
                // Show configuration file path
                let config_path = Config::config_path()
                    .map_err(|e| anyhow::anyhow!("Failed to get config path: {}", e))?;
                println!("{}", config_path.display());
            } else {
                eprintln!("Please specify --show, --edit, or --path");
            }
        }
        
        Commands::Sync {
            source,
            target,
            algo,
            level,
            threads,
            follow_symlinks,
            full,
        } => {
            info!("Synchronizing {:?} to {:?}", source, target);
            
            if !source.is_dir() {
                error!("Source must be a directory");
                return Err(anyhow::anyhow!("Source must be a directory"));
            }
            
            // Determine manifest path
            let manifest_path = target.with_extension("fluxmanifest");
            
            if full || !manifest_path.exists() {
                // Full backup
                info!("Performing full backup (no previous manifest found or --full specified)");
                
                let options = flux_core::archive::PackOptions {
                    smart: false,
                    algorithm: algo,
                    level,
                    threads,
                    force_compress: false,
                    follow_symlinks,
                };
                
                // Use tar.gz as default format for sync
                let format = Some("tar.gz");
                flux_core::archive::pack_with_strategy(&source, &target, format, options)?;
                
                // Generate and save manifest
                let manifest = flux_core::manifest::Manifest::from_directory(&source)?;
                manifest.save(&manifest_path)?;
                
                info!("Full backup complete. Manifest saved to: {:?}", manifest_path);
            } else {
                // Incremental backup
                info!("Performing incremental backup using manifest: {:?}", manifest_path);
                
                let (new_manifest_path, diff) = flux_core::archive::incremental::pack_incremental(
                    &source,
                    &target,
                    &manifest_path,
                    flux_core::archive::PackOptions {
                        smart: false,
                        algorithm: algo,
                        level,
                        threads,
                        force_compress: false,
                        follow_symlinks,
                    },
                )?;
                
                if diff.has_changes() {
                    info!("Incremental backup complete");
                    info!("Changes: {} added, {} modified, {} deleted", 
                        diff.added.len(), diff.modified.len(), diff.deleted.len());
                    info!("Updated manifest: {:?}", new_manifest_path);
                } else {
                    info!("No changes detected since last backup");
                }
            }
        }
    }

    Ok(())
}

/// Print entries as a tree structure
fn print_tree(entries: &[flux_core::archive::ArchiveEntry]) {
    // Simple tree printing
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));
    
    println!("Archive contents:");
    for entry in &sorted_entries {
        let depth = entry.path.components().count();
        let indent = "  ".repeat(depth.saturating_sub(1));
        
        let icon = if entry.is_dir {
            "📁"
        } else if entry.is_symlink {
            "🔗"
        } else {
            "📄"
        };
        
        let name = entry.path.to_string_lossy();
        println!("{}{} {}", indent, icon, name);
    }
}

/// Map errors to exit codes according to requirements:
/// - 0: Success
/// - 1: General error
/// - 2: IO error
/// - 3: Invalid arguments
/// - 4: Partial failure
fn map_error_to_exit_code(err: &anyhow::Error) -> i32 {
    // Check if it's a flux_core error
    if let Some(flux_err) = err.downcast_ref::<flux_core::Error>() {
        match flux_err {
            flux_core::Error::Io(_) => 2,
            flux_core::Error::InvalidPath(_) => 3,
            flux_core::Error::UnsupportedFormat(_) => 3,
            flux_core::Error::Archive(_) => 4,
            flux_core::Error::Compression(_) => 4,
            flux_core::Error::Config(_) | flux_core::Error::ConfigError(_) => 1,
            flux_core::Error::Other(_) => 1,
            flux_core::Error::Zip(_) => 4,
            flux_core::Error::ArchiveError(_) => 4,
            flux_core::Error::FileExists(_) => 3,
            flux_core::Error::UnsupportedOperation(_) => 3,
            flux_core::Error::PartialFailure { .. } => 4,
            flux_core::Error::NotFound(_) => 2,
            flux_core::Error::SecurityError(_) => 3,
        }
    } else if err.is::<std::io::Error>() {
        2
    } else if err.to_string().contains("argument") || err.to_string().contains("invalid") {
        3
    } else {
        1
    }
}
