//! Utility functions for flux-core

use std::fs;
use std::path::Path;

/// Calculate the total size of a path (file or directory) in bytes
///
/// # Arguments
/// * `path` - The path to calculate size for
///
/// # Returns
/// The total size in bytes, or 0 if the path cannot be accessed
pub fn calculate_path_size<P: AsRef<Path>>(path: P) -> u64 {
    let path = path.as_ref();

    if path.is_file() {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else if path.is_dir() {
        let mut size = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                size += calculate_path_size(entry.path());
            }
        }
        size
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"Hello, World!";
        fs::write(&file_path, content).unwrap();

        assert_eq!(calculate_path_size(&file_path), content.len() as u64);
    }

    #[test]
    fn test_calculate_directory_size() {
        let temp_dir = TempDir::new().unwrap();

        // Create some files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file3 = subdir.join("file3.txt");

        fs::write(&file1, b"Hello").unwrap();
        fs::write(&file2, b"World").unwrap();
        fs::write(&file3, b"!").unwrap();

        let total_size = calculate_path_size(temp_dir.path());
        assert_eq!(total_size, 11); // "Hello" + "World" + "!"
    }

    #[test]
    fn test_nonexistent_path() {
        assert_eq!(calculate_path_size("/nonexistent/path"), 0);
    }
}
