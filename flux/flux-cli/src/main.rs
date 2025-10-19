use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

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
    },

    /// Inspect archive contents
    Inspect {
        /// Archive file to inspect
        archive: PathBuf,

        /// Output format as JSON
        #[arg(long)]
        json: bool,
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
        } => {
            info!("Extracting archive: {:?}", archive);
            let output_dir = output.unwrap_or_else(|| PathBuf::from("."));
            
            let options = flux_lib::archive::ExtractOptions {
                overwrite,
                skip,
                rename,
                strip_components,
            };
            
            flux_lib::archive::extract_with_options(&archive, &output_dir, options)?;
            info!("Extraction complete");
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
        } => {
            info!("Packing {:?} into {:?}", input, output);
            
            let options = flux_lib::archive::PackOptions {
                smart,
                algorithm: algo,
                level,
                threads,
                force_compress,
                follow_symlinks,
            };
            
            flux_lib::archive::pack_with_strategy(&input, &output, format.as_deref(), options)?;
            info!("Packing complete");
        }

        Commands::Inspect { archive, json } => {
            info!("Inspecting archive: {:?}", archive);
            
            let entries = flux_lib::inspect(&archive)?;
            
            if json {
                // Output as JSON
                let json_output = serde_json::to_string_pretty(&entries)?;
                println!("{}", json_output);
            } else {
                // Output as human-readable table
                println!("{:<50} {:>15} {:>15} {:>10} {:>20}", 
                    "Path", "Size", "Compressed", "Mode", "Modified");
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
                    
                    let compressed_str = entry.compressed_size
                        .map(|s| format!("{}", s))
                        .unwrap_or_else(|| "-".to_string());
                    
                    println!("{:<50} {:>15} {:>15} {:>10} {:>20}",
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
            if show {
                eprintln!("Config show not yet implemented");
            } else if edit {
                eprintln!("Config edit not yet implemented");
            } else if path {
                eprintln!("Config path not yet implemented");
            } else {
                eprintln!("Please specify --show, --edit, or --path");
            }
        }
    }

    Ok(())
}

/// Map errors to exit codes according to requirements:
/// - 0: Success
/// - 1: General error
/// - 2: IO error
/// - 3: Invalid arguments
/// - 4: Partial failure
fn map_error_to_exit_code(err: &anyhow::Error) -> i32 {
    // Check if it's a flux_lib error
    if let Some(flux_err) = err.downcast_ref::<flux_lib::Error>() {
        match flux_err {
            flux_lib::Error::Io(_) => 2,
            flux_lib::Error::InvalidPath(_) => 3,
            flux_lib::Error::UnsupportedFormat(_) => 3,
            flux_lib::Error::Archive(_) => 4,
            flux_lib::Error::Compression(_) => 4,
            flux_lib::Error::Config(_) | flux_lib::Error::ConfigError(_) => 1,
            flux_lib::Error::Other(_) => 1,
        }
    } else if err.is::<std::io::Error>() {
        2
    } else if err.to_string().contains("argument") || err.to_string().contains("invalid") {
        3
    } else {
        1
    }
}
