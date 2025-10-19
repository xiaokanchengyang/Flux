//! Archive operations module

pub mod tar;

use crate::{Error, Result};
use std::path::Path;

/// Pack files into an archive
pub fn pack<P: AsRef<Path>, Q: AsRef<Path>>(
    input: P,
    output: Q,
    format: Option<&str>,
) -> Result<()> {
    let input = input.as_ref();
    let output = output.as_ref();

    // For now, we only support tar format
    let format = format.unwrap_or("tar");

    match format {
        "tar" => tar::pack_tar(input, output),
        _ => Err(Error::UnsupportedFormat(format.to_string())),
    }
}

/// Extract files from an archive
pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(archive: P, output_dir: Q) -> Result<()> {
    let archive = archive.as_ref();
    let output_dir = output_dir.as_ref();

    // For now, detect format by extension
    let format = archive
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match format {
        "tar" => tar::extract_tar(archive, output_dir),
        _ => Err(Error::UnsupportedFormat(format.to_string())),
    }
}
