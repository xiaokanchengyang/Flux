//! Common assertions for flux testing

use std::path::Path;
use walkdir::WalkDir;
use anyhow::Result;

/// Asserts that two directory structures are identical
pub fn assert_dirs_equal(dir1: &Path, dir2: &Path) -> Result<()> {
    let entries1 = collect_entries(dir1)?;
    let entries2 = collect_entries(dir2)?;
    
    assert_eq!(
        entries1.len(),
        entries2.len(),
        "Different number of entries: {} vs {}",
        entries1.len(),
        entries2.len()
    );
    
    for (path1, path2) in entries1.iter().zip(entries2.iter()) {
        assert_eq!(
            path1.file_name(),
            path2.file_name(),
            "Different file names"
        );
        
        let meta1 = std::fs::metadata(path1)?;
        let meta2 = std::fs::metadata(path2)?;
        
        assert_eq!(
            meta1.is_file(),
            meta2.is_file(),
            "File type mismatch for {:?}",
            path1.file_name()
        );
        
        if meta1.is_file() {
            let content1 = std::fs::read(path1)?;
            let content2 = std::fs::read(path2)?;
            assert_eq!(
                content1,
                content2,
                "Content mismatch for {:?}",
                path1.file_name()
            );
        }
    }
    
    Ok(())
}

/// Asserts that a file has specific permissions (Unix only)
#[cfg(unix)]
pub fn assert_file_permissions(path: &Path, expected: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    
    let metadata = std::fs::metadata(path)?;
    let permissions = metadata.permissions();
    let mode = permissions.mode() & 0o777;
    
    assert_eq!(
        mode, expected,
        "Permission mismatch for {:?}: expected {:o}, got {:o}",
        path, expected, mode
    );
    
    Ok(())
}

fn collect_entries(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut entries: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .collect();
    
    entries.sort();
    Ok(entries)
}