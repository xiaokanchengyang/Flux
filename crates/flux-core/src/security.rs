//! Security utilities for safe archive operations

use crate::{Error, Result};
use std::path::{Component, Path, PathBuf};
use tracing::{error, warn};

/// Maximum allowed extraction size (10 GB by default)
pub const DEFAULT_MAX_EXTRACTION_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Maximum compression ratio to detect potential zip bombs
pub const DEFAULT_MAX_COMPRESSION_RATIO: f64 = 100.0;

/// Security options for archive operations
#[derive(Debug, Clone)]
pub struct SecurityOptions {
    /// Maximum total size allowed for extraction
    pub max_extraction_size: u64,
    /// Maximum compression ratio allowed (uncompressed/compressed)
    pub max_compression_ratio: f64,
    /// Whether to allow symlinks that point outside the extraction directory
    pub allow_external_symlinks: bool,
    /// Whether to check available disk space before extraction
    pub check_disk_space: bool,
}

impl Default for SecurityOptions {
    fn default() -> Self {
        Self {
            max_extraction_size: DEFAULT_MAX_EXTRACTION_SIZE,
            max_compression_ratio: DEFAULT_MAX_COMPRESSION_RATIO,
            allow_external_symlinks: false,
            check_disk_space: true,
        }
    }
}

/// Sanitize and validate a path to prevent directory traversal attacks
pub fn sanitize_path(base: &Path, untrusted: &Path) -> Result<PathBuf> {
    let mut result = base.to_path_buf();

    // Iterate through components and build safe path
    for component in untrusted.components() {
        match component {
            Component::Normal(name) => {
                result.push(name);
            }
            Component::CurDir => {
                // Current directory "." is safe, just skip
            }
            Component::ParentDir => {
                // Parent directory ".." is dangerous - could escape extraction directory
                error!(path = ?untrusted, "Path contains parent directory component");
                return Err(Error::InvalidPath(format!(
                    "Path traversal attempt detected: {:?}",
                    untrusted
                )));
            }
            Component::RootDir => {
                // Absolute paths are not allowed
                error!(path = ?untrusted, "Path is absolute");
                return Err(Error::InvalidPath(format!(
                    "Absolute path not allowed: {:?}",
                    untrusted
                )));
            }
            Component::Prefix(_) => {
                // Windows drive prefixes not allowed
                error!(path = ?untrusted, "Path contains Windows prefix");
                return Err(Error::InvalidPath(format!(
                    "Windows path prefix not allowed: {:?}",
                    untrusted
                )));
            }
        }
    }

    // Verify the final path is still within the base directory
    let canonical_base = base
        .canonicalize()
        .map_err(|e| Error::InvalidPath(format!("Cannot canonicalize base path: {}", e)))?;

    // Check if result starts with base (without canonicalizing result since it may not exist yet)
    if !result.starts_with(&canonical_base) {
        error!(base = ?base, path = ?untrusted, result = ?result, "Path escapes base directory");
        return Err(Error::InvalidPath(format!(
            "Path would escape extraction directory: {:?}",
            untrusted
        )));
    }

    Ok(result)
}

/// Validate symlink target to prevent escaping extraction directory
pub fn validate_symlink(
    base: &Path,
    link_path: &Path,
    target: &Path,
    allow_external: bool,
) -> Result<()> {
    if allow_external {
        return Ok(());
    }

    // If target is absolute, it's definitely external
    if target.is_absolute() {
        warn!(link = ?link_path, target = ?target, "Symlink has absolute target");
        return Err(Error::InvalidPath(format!(
            "Symlink points outside extraction directory: {:?} -> {:?}",
            link_path, target
        )));
    }

    // Resolve the link target relative to the link's parent directory
    let link_parent = link_path
        .parent()
        .ok_or_else(|| Error::InvalidPath("Symlink has no parent directory".to_string()))?;

    // Normalize the path by resolving .. components
    let mut normalized = PathBuf::new();
    let start_path = if link_parent.starts_with(base) {
        link_parent.strip_prefix(base).unwrap_or(link_parent)
    } else {
        link_parent
    };

    // Start with the link's parent directory components
    for component in start_path.components() {
        if let Component::Normal(name) = component {
            normalized.push(name)
        }
    }

    // Apply the target path components
    for component in target.components() {
        match component {
            Component::ParentDir => {
                if !normalized.pop() {
                    // Trying to go above the base directory
                    warn!(link = ?link_path, target = ?target, "Symlink target escapes extraction directory");
                    return Err(Error::InvalidPath(format!(
                        "Symlink target would escape extraction directory: {:?} -> {:?}",
                        link_path, target
                    )));
                }
            }
            Component::Normal(name) => {
                normalized.push(name);
            }
            Component::CurDir => {
                // Current directory, no change
            }
            _ => {
                // Unexpected component
                warn!(link = ?link_path, target = ?target, component = ?component, "Unexpected path component in symlink target");
                return Err(Error::InvalidPath(format!(
                    "Invalid symlink target: {:?} -> {:?}",
                    link_path, target
                )));
            }
        }
    }

    Ok(())
}

/// Check for potential zip bomb by validating compression ratio
pub fn check_compression_ratio(
    compressed_size: u64,
    uncompressed_size: u64,
    max_ratio: f64,
) -> Result<()> {
    if compressed_size == 0 {
        return Ok(());
    }

    let ratio = uncompressed_size as f64 / compressed_size as f64;
    if ratio > max_ratio {
        error!(
            compressed_size,
            uncompressed_size, ratio, max_ratio, "Suspicious compression ratio detected"
        );
        return Err(Error::SecurityError(format!(
            "Suspicious compression ratio {:.1}:1 exceeds maximum {:.1}:1",
            ratio, max_ratio
        )));
    }

    Ok(())
}

/// Check if extraction would exceed size limits
pub fn check_extraction_size(current_total: u64, entry_size: u64, max_size: u64) -> Result<()> {
    let new_total = current_total.saturating_add(entry_size);
    if new_total > max_size {
        error!(
            current_total,
            entry_size, new_total, max_size, "Extraction would exceed size limit"
        );
        return Err(Error::SecurityError(format!(
            "Extraction would exceed maximum size of {} bytes",
            max_size
        )));
    }

    Ok(())
}

/// Check available disk space
pub fn check_disk_space(path: &Path, required_bytes: u64) -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs;

        let _metadata = fs::metadata(path).or_else(|_| {
            // If path doesn't exist, check parent directory
            path.parent()
                .ok_or_else(|| Error::InvalidPath("No parent directory".to_string()))
                .and_then(|p| fs::metadata(p).map_err(Error::Io))
        })?;

        // Get filesystem statistics
        let stat = unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
                .map_err(|_| Error::InvalidPath("Invalid path for statvfs".to_string()))?;

            if libc::statvfs(path_cstr.as_ptr(), &mut stat) != 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }
            stat
        };

        let available = stat.f_bavail as u64 * stat.f_frsize as u64;
        if available < required_bytes {
            error!(
                available,
                required_bytes,
                path = ?path,
                "Insufficient disk space"
            );
            return Err(Error::SecurityError(format!(
                "Insufficient disk space: {} bytes available, {} bytes required",
                available, required_bytes
            )));
        }
    }

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        use winapi::um::winnt::ULARGE_INTEGER;

        let path_wide: Vec<u16> = OsStr::new(&path.to_string_lossy())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut available = ULARGE_INTEGER::default();
        let mut total = ULARGE_INTEGER::default();
        let mut free = ULARGE_INTEGER::default();

        unsafe {
            if GetDiskFreeSpaceExW(path_wide.as_ptr(), &mut available, &mut total, &mut free) == 0 {
                return Err(Error::Io(std::io::Error::last_os_error().into()));
            }
        }

        let available_bytes = unsafe { *available.QuadPart() } as u64;
        if available_bytes < required_bytes {
            error!(
                available_bytes,
                required_bytes,
                path = ?path,
                "Insufficient disk space"
            );
            return Err(Error::SecurityError(format!(
                "Insufficient disk space: {} bytes available, {} bytes required",
                available_bytes, required_bytes
            )));
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        // On other platforms, skip disk space check
        warn!("Disk space check not implemented for this platform");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_path_normal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let path = Path::new("subdir/file.txt");
        let result = sanitize_path(base, path).unwrap();
        assert_eq!(result, base.join("subdir/file.txt"));
    }

    #[test]
    fn test_sanitize_path_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let path = Path::new("../etc/passwd");
        assert!(sanitize_path(base, path).is_err());
    }

    #[test]
    fn test_sanitize_path_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let path = Path::new("/etc/passwd");
        assert!(sanitize_path(base, path).is_err());
    }

    #[test]
    fn test_compression_ratio_normal() {
        assert!(check_compression_ratio(1000, 5000, 100.0).is_ok());
    }

    #[test]
    fn test_compression_ratio_suspicious() {
        assert!(check_compression_ratio(100, 1_000_000, 100.0).is_err());
    }

    #[test]
    fn test_extraction_size_within_limit() {
        assert!(check_extraction_size(1000, 500, 2000).is_ok());
    }

    #[test]
    fn test_extraction_size_exceeds_limit() {
        assert!(check_extraction_size(1000, 1500, 2000).is_err());
    }
}
