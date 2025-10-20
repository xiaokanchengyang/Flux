//! Testing utilities and fixtures for flux
//! 
//! This crate provides common testing utilities, fixtures, and helpers
//! for testing flux-based applications and libraries.

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use anyhow::Result;

pub mod fixtures;
pub mod assertions;
pub mod helpers;

/// Creates a temporary test directory with cleanup on drop
pub struct TestDir {
    dir: TempDir,
}

impl TestDir {
    /// Creates a new temporary test directory
    pub fn new() -> Result<Self> {
        Ok(Self {
            dir: TempDir::new()?,
        })
    }

    /// Returns the path to the temporary directory
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Creates a file with the given name and content in the test directory
    pub fn create_file(&self, name: &str, content: &[u8]) -> Result<PathBuf> {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Creates a directory with the given name in the test directory
    pub fn create_dir(&self, name: &str) -> Result<PathBuf> {
        let path = self.dir.path().join(name);
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_dir() {
        let test_dir = TestDir::new().unwrap();
        assert!(test_dir.path().exists());
    }

    #[test]
    fn test_create_file() {
        let test_dir = TestDir::new().unwrap();
        let file_path = test_dir.create_file("test.txt", b"Hello, World!").unwrap();
        assert!(file_path.exists());
        assert_eq!(std::fs::read(&file_path).unwrap(), b"Hello, World!");
    }
}