//! Manifest handling for incremental backups

use crate::{Error, Result};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, info};
use walkdir::WalkDir;

/// File entry in manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from base directory
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Blake3 hash of file contents
    pub hash: String,
    /// Modified time (Unix timestamp)
    pub mtime: i64,
    /// Unix permissions (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
    /// Link target (for symlinks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<PathBuf>,
}

/// Backup manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Version of the manifest format
    pub version: u32,
    /// Creation timestamp
    pub created: i64,
    /// Base directory (for reference)
    pub base_dir: PathBuf,
    /// Total size of all files
    pub total_size: u64,
    /// Number of files
    pub file_count: u32,
    /// File entries indexed by path
    pub files: HashMap<PathBuf, FileEntry>,
}

impl Manifest {
    /// Current manifest version
    const VERSION: u32 = 1;

    /// Create a new manifest for a directory
    pub fn from_directory<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        let mut files = HashMap::new();
        let mut total_size = 0u64;
        let mut file_count = 0u32;

        info!("Creating manifest for directory: {:?}", base_dir);

        for entry in WalkDir::new(base_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_dir)
                .map_err(|_| Error::InvalidPath("Failed to compute relative path".to_string()))?;

            // Skip empty relative paths (the base directory itself)
            if relative_path.as_os_str().is_empty() {
                continue;
            }

            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let is_symlink = metadata.is_symlink();

            let entry = if is_symlink {
                let link_target = fs::read_link(path)?;
                FileEntry {
                    path: relative_path.to_path_buf(),
                    size: 0,
                    hash: String::new(),
                    mtime: metadata
                        .modified()
                        .map(|t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64
                        })
                        .unwrap_or(0),
                    mode: get_file_mode(&metadata),
                    is_dir: false,
                    is_symlink: true,
                    link_target: Some(link_target),
                }
            } else if is_dir {
                FileEntry {
                    path: relative_path.to_path_buf(),
                    size: 0,
                    hash: String::new(),
                    mtime: metadata
                        .modified()
                        .map(|t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64
                        })
                        .unwrap_or(0),
                    mode: get_file_mode(&metadata),
                    is_dir: true,
                    is_symlink: false,
                    link_target: None,
                }
            } else {
                // Regular file - compute hash
                let size = metadata.len();
                let hash = compute_file_hash(path)?;

                total_size += size;
                file_count += 1;

                FileEntry {
                    path: relative_path.to_path_buf(),
                    size,
                    hash,
                    mtime: metadata
                        .modified()
                        .map(|t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64
                        })
                        .unwrap_or(0),
                    mode: get_file_mode(&metadata),
                    is_dir: false,
                    is_symlink: false,
                    link_target: None,
                }
            };

            debug!("Added to manifest: {:?}", entry.path);
            files.insert(relative_path.to_path_buf(), entry);
        }

        Ok(Self {
            version: Self::VERSION,
            created: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            base_dir: base_dir.to_path_buf(),
            total_size,
            file_count,
            files,
        })
    }

    /// Save manifest to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Other(format!("Failed to serialize manifest: {}", e)))?;

        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;

        info!("Saved manifest to: {:?}", path);
        Ok(())
    }

    /// Load manifest from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let manifest: Self = serde_json::from_str(&contents)
            .map_err(|e| Error::Other(format!("Failed to parse manifest: {}", e)))?;

        if manifest.version != Self::VERSION {
            return Err(Error::Other(format!(
                "Unsupported manifest version: {} (expected {})",
                manifest.version,
                Self::VERSION
            )));
        }

        info!("Loaded manifest from: {:?}", path);
        Ok(manifest)
    }

    /// Compare with another manifest to find changes
    pub fn diff(&self, other: &Manifest) -> ManifestDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Find added and modified files
        for (path, entry) in &other.files {
            match self.files.get(path) {
                Some(old_entry) => {
                    // Check if modified
                    if entry.hash != old_entry.hash || entry.mtime != old_entry.mtime {
                        modified.push(path.clone());
                    }
                }
                None => {
                    // New file
                    added.push(path.clone());
                }
            }
        }

        // Find deleted files
        for path in self.files.keys() {
            if !other.files.contains_key(path) {
                deleted.push(path.clone());
            }
        }

        ManifestDiff {
            added,
            modified,
            deleted,
        }
    }
}

/// Differences between two manifests
#[derive(Debug, Clone)]
pub struct ManifestDiff {
    /// Files added in the new manifest
    pub added: Vec<PathBuf>,
    /// Files modified in the new manifest
    pub modified: Vec<PathBuf>,
    /// Files deleted from the old manifest
    pub deleted: Vec<PathBuf>,
}

impl ManifestDiff {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.modified.is_empty() || !self.deleted.is_empty()
    }

    /// Get total number of changes
    pub fn change_count(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

/// Compute Blake3 hash of a file
fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Get file mode (Unix permissions)
#[cfg(unix)]
fn get_file_mode(metadata: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode())
}

#[cfg(not(unix))]
fn get_file_mode(_metadata: &std::fs::Metadata) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("subdir/file2.txt");

        fs::create_dir_all(file1.parent().unwrap()).unwrap();
        fs::create_dir_all(file2.parent().unwrap()).unwrap();

        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let manifest = Manifest::from_directory(temp_dir.path()).unwrap();

        assert_eq!(manifest.file_count, 2);
        assert!(manifest.files.contains_key(Path::new("file1.txt")));
        assert!(manifest.files.contains_key(Path::new("subdir/file2.txt")));
    }

    #[test]
    fn test_manifest_diff() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");

        fs::write(&file1, "content1").unwrap();
        let manifest1 = Manifest::from_directory(temp_dir.path()).unwrap();

        // Modify file
        fs::write(&file1, "content2").unwrap();

        // Add new file
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file2, "new file").unwrap();

        let manifest2 = Manifest::from_directory(temp_dir.path()).unwrap();

        let diff = manifest1.diff(&manifest2);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.deleted.len(), 0);
    }
}
