use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, cli.quiet);

    match cli.command {
        Commands::Extract {
            archive,
            output,
            overwrite: _,
            skip: _,
            rename: _,
            strip_components: _,
        } => {
            info!("Extracting archive: {:?}", archive);
            let output_dir = output.unwrap_or_else(|| PathBuf::from("."));
            flux_lib::extract(&archive, &output_dir)?;
            info!("Extraction complete");
        }

        Commands::Pack {
            input,
            output,
            format,
            smart: _,
            algo: _,
            level: _,
            threads: _,
        } => {
            info!("Packing {:?} into {:?}", input, output);
            flux_lib::pack(&input, &output, format.as_deref())?;
            info!("Packing complete");
        }

        Commands::Inspect { archive, json: _ } => {
            info!("Inspecting archive: {:?}", archive);
            // TODO: Implement inspect functionality
            eprintln!("Inspect functionality not yet implemented");
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
