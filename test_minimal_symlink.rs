use std::fs;
use std::os::unix::fs as unix_fs;
use tempfile::TempDir;
use flux_core::archive::tar::pack_tar_with_options;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create a simple file
    fs::write(base.join("file.txt"), "content").unwrap();
    
    // Test 1: Pack without symlinks - should work
    let archive1 = base.join("test1.tar");
    match pack_tar_with_options(&base, &archive1, false) {
        Ok(_) => println!("Test 1 passed: Pack without symlinks"),
        Err(e) => println!("Test 1 failed: {}", e),
    }

    // Create a symlink loop: a -> b -> a
    let link_a = base.join("link_a");
    let link_b = base.join("link_b");
    unix_fs::symlink(&link_b, &link_a).unwrap();
    unix_fs::symlink(&link_a, &link_b).unwrap();

    // Test 2: Pack with symlinks but follow_symlinks=false - should work
    let archive2 = base.join("test2.tar");
    match pack_tar_with_options(&base, &archive2, false) {
        Ok(_) => println!("Test 2 passed: Pack with symlinks (not following)"),
        Err(e) => println!("Test 2 failed: {}", e),
    }

    // Test 3: Pack with symlinks and follow_symlinks=true - should handle gracefully
    let archive3 = base.join("test3.tar");
    println!("Starting test 3...");
    match pack_tar_with_options(&base, &archive3, true) {
        Ok(_) => println!("Test 3 passed: Pack with symlinks (following)"),
        Err(e) => println!("Test 3 failed: {}", e),
    }
}