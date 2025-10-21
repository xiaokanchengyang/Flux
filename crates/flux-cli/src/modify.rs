//! Archive modification commands for CLI

use clap::{Args, Subcommand};
use flux_core::archive::modifier::{create_modifier, ModifyOptions};
use flux_core::Result;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Subcommand)]
pub enum ModifyCommand {
    /// Add files to an archive
    Add(AddArgs),
    /// Remove files from an archive
    Remove(RemoveArgs),
    /// Update files in an archive
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Archive to modify
    #[arg(value_name = "ARCHIVE")]
    pub archive: PathBuf,
    
    /// Files to add
    #[arg(value_name = "FILES", required = true)]
    pub files: Vec<PathBuf>,
    
    /// Compression level (0-9)
    #[arg(short = 'l', long, default_value = "6")]
    pub level: u32,
    
    /// Don't preserve file permissions
    #[arg(long)]
    pub no_preserve_perms: bool,
    
    /// Don't preserve timestamps
    #[arg(long)]
    pub no_preserve_time: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Archive to modify
    #[arg(value_name = "ARCHIVE")]
    pub archive: PathBuf,
    
    /// Patterns of files to remove (supports * wildcard)
    #[arg(value_name = "PATTERNS", required = true)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Archive to modify
    #[arg(value_name = "ARCHIVE")]
    pub archive: PathBuf,
    
    /// Files to update
    #[arg(value_name = "FILES", required = true)]
    pub files: Vec<PathBuf>,
    
    /// Compression level (0-9)
    #[arg(short = 'l', long, default_value = "6")]
    pub level: u32,
}

pub fn execute_modify(command: ModifyCommand) -> Result<()> {
    match command {
        ModifyCommand::Add(args) => add_files(args),
        ModifyCommand::Remove(args) => remove_files(args),
        ModifyCommand::Update(args) => update_files(args),
    }
}

fn add_files(args: AddArgs) -> Result<()> {
    if !args.archive.exists() {
        return Err(flux_core::Error::NotFound(format!(
            "Archive not found: {:?}",
            args.archive
        )));
    }
    
    let options = ModifyOptions {
        preserve_permissions: !args.no_preserve_perms,
        preserve_timestamps: !args.no_preserve_time,
        follow_symlinks: false,
        compression_level: args.level,
    };
    
    info!("Adding {} files to {:?}", args.files.len(), args.archive);
    
    let modifier = create_modifier(&args.archive)?;
    modifier.add_files(&args.archive, &args.files, &options)?;
    
    println!("Successfully added {} files to archive", args.files.len());
    Ok(())
}

fn remove_files(args: RemoveArgs) -> Result<()> {
    if !args.archive.exists() {
        return Err(flux_core::Error::NotFound(format!(
            "Archive not found: {:?}",
            args.archive
        )));
    }
    
    let options = ModifyOptions::default();
    
    info!("Removing files matching patterns from {:?}", args.archive);
    
    let modifier = create_modifier(&args.archive)?;
    modifier.remove_files(&args.archive, &args.patterns, &options)?;
    
    println!("Successfully removed files from archive");
    Ok(())
}

fn update_files(args: UpdateArgs) -> Result<()> {
    if !args.archive.exists() {
        return Err(flux_core::Error::NotFound(format!(
            "Archive not found: {:?}",
            args.archive
        )));
    }
    
    let options = ModifyOptions {
        preserve_permissions: true,
        preserve_timestamps: true,
        follow_symlinks: false,
        compression_level: args.level,
    };
    
    info!("Updating {} files in {:?}", args.files.len(), args.archive);
    
    let modifier = create_modifier(&args.archive)?;
    modifier.update_files(&args.archive, &args.files, &options)?;
    
    println!("Successfully updated {} files in archive", args.files.len());
    Ok(())
}