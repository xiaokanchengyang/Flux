//! Metadata preservation utilities

use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;

/// Metadata to preserve during archiving
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    #[cfg(unix)]
    pub mode: Option<u32>,
    #[cfg(unix)]
    pub uid: Option<u32>,
    #[cfg(unix)]
    pub gid: Option<u32>,
}

impl FileMetadata {
    /// Extract metadata from a file
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let metadata = path.metadata()?;
        Self::from_metadata(&metadata)
    }

    /// Extract metadata from std::fs::Metadata
    pub fn from_metadata(metadata: &Metadata) -> std::io::Result<Self> {
        Ok(Self {
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            created: metadata.created().ok(),
            #[cfg(unix)]
            mode: {
                use std::os::unix::fs::MetadataExt;
                Some(metadata.mode())
            },
            #[cfg(unix)]
            uid: {
                use std::os::unix::fs::MetadataExt;
                Some(metadata.uid())
            },
            #[cfg(unix)]
            gid: {
                use std::os::unix::fs::MetadataExt;
                Some(metadata.gid())
            },
        })
    }

    /// Apply metadata to a file
    pub fn apply_to_path(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = self.mode {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
            }
        }

        // Set timestamps if available
        if let (Some(accessed), Some(modified)) = (self.accessed, self.modified) {
            filetime::set_file_times(
                path,
                filetime::FileTime::from_system_time(accessed),
                filetime::FileTime::from_system_time(modified),
            )
            .ok(); // Ignore errors for timestamps
        }

        Ok(())
    }
}
