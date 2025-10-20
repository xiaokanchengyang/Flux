//! Helper utilities for flux testing

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Runs a flux CLI command and returns the output
pub fn run_flux_cli(args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new("flux").args(args).output()?;

    Ok(output)
}

/// Creates a test archive using flux CLI
pub fn create_test_archive(input_dir: &Path, output_file: &Path) -> Result<()> {
    let output = run_flux_cli(&[
        "pack",
        input_dir.to_str().unwrap(),
        "-o",
        output_file.to_str().unwrap(),
    ])?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create archive: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Extracts a test archive using flux CLI
pub fn extract_test_archive(archive_file: &Path, output_dir: &Path) -> Result<()> {
    let output = run_flux_cli(&[
        "extract",
        archive_file.to_str().unwrap(),
        "-o",
        output_dir.to_str().unwrap(),
    ])?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to extract archive: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Gets file metadata preservation settings for the current platform
pub fn get_metadata_preservation_support() -> MetadataSupport {
    MetadataSupport {
        timestamps: true,
        permissions: cfg!(unix),
        ownership: cfg!(unix) && std::env::var("USER").unwrap_or_default() == "root",
        extended_attrs: cfg!(target_os = "macos") || cfg!(target_os = "linux"),
    }
}

#[derive(Debug, Clone)]
pub struct MetadataSupport {
    pub timestamps: bool,
    pub permissions: bool,
    pub ownership: bool,
    pub extended_attrs: bool,
}
