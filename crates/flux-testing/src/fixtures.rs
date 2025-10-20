//! Common test fixtures for flux testing

use crate::TestDir;
use anyhow::Result;

/// Creates a standard test file structure
pub fn create_test_files(test_dir: &TestDir) -> Result<()> {
    // Text files
    test_dir.create_file("file1.txt", b"This is file 1 content.")?;
    test_dir.create_file("file2.txt", b"This is file 2 content.")?;

    // Directory structure
    test_dir.create_dir("subdir")?;
    test_dir.create_file("subdir/file3.txt", b"This is file 3 in subdir.")?;

    // Binary file (simple image placeholder)
    test_dir.create_file("image.jpg", &[0xFF, 0xD8, 0xFF, 0xE0])?;

    // Large file
    let large_content = "x".repeat(1024 * 1024); // 1MB
    test_dir.create_file("large.log", large_content.as_bytes())?;

    Ok(())
}

/// Creates a test archive structure
pub fn create_archive_structure(test_dir: &TestDir) -> Result<()> {
    create_test_files(test_dir)?;

    // Additional archive-specific files
    test_dir.create_file("README.md", b"# Test Archive\n\nThis is a test archive.")?;
    test_dir.create_file(".gitignore", b"target/\n*.tmp")?;

    // Nested directories
    test_dir.create_dir("src")?;
    test_dir.create_file("src/main.rs", b"fn main() {}")?;
    test_dir.create_dir("src/modules")?;
    test_dir.create_file("src/modules/mod.rs", b"pub mod utils;")?;

    Ok(())
}

/// Creates a symlink test structure (Unix only)
#[cfg(unix)]
pub fn create_symlink_structure(test_dir: &TestDir) -> Result<()> {
    use std::os::unix::fs::symlink;

    // Create base files
    let file1 = test_dir.create_file("file1.txt", b"Original file")?;
    test_dir.create_dir("subdir")?;

    // Create symlinks
    symlink(&file1, test_dir.path().join("link_to_file1.txt"))?;
    symlink(
        "../file1.txt",
        test_dir.path().join("subdir/link_to_parent_file.txt"),
    )?;

    Ok(())
}
