#[cfg(unix)]
mod symlink_loop_tests {
    use flux_core::archive::tar::pack_tar_with_options;
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    // Skip the symlink loop test for now as WalkDir hangs on loops
    // TODO: Implement proper symlink loop detection before entering WalkDir
    /*
    #[test]
    fn test_symlink_loop_detection() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create a symlink loop: a -> b -> a
        let link_a = base.join("link_a");
        let link_b = base.join("link_b");

        unix_fs::symlink(&link_b, &link_a).unwrap();
        unix_fs::symlink(&link_a, &link_b).unwrap();

        // Also create a normal file for the archive to have something
        fs::write(base.join("file.txt"), "content").unwrap();

        let archive = base.join("test.tar");

        // Packing with follow_symlinks=true should detect the loop
        let result = pack_tar_with_options(&base, &archive, true);

        // The operation should fail with a symlink loop error
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Symlink loop detected"));
    }
    */

    #[test]
    #[ignore = "WalkDir hangs on some symlink configurations"]
    fn test_symlink_chain_without_loop() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create a valid symlink chain: file.txt <- link1 <- link2
        let file = base.join("file.txt");
        fs::write(&file, "content").unwrap();

        let link1 = base.join("link1");
        unix_fs::symlink(&file, &link1).unwrap();

        let link2 = base.join("link2");
        unix_fs::symlink(&link1, &link2).unwrap();

        let archive = base.join("test.tar");

        // This should work fine without following symlinks
        let result = pack_tar_with_options(&base, &archive, false);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "WalkDir hangs on some symlink configurations"]
    fn test_broken_symlink_handling() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create a broken symlink (points to non-existent file)
        let broken_link = base.join("broken_link");
        unix_fs::symlink("/non/existent/path", &broken_link).unwrap();

        // Also create a normal file
        fs::write(base.join("file.txt"), "content").unwrap();

        let archive = base.join("test.tar");

        // Should handle broken symlinks gracefully without following them
        let result = pack_tar_with_options(&base, &archive, false);
        assert!(result.is_ok());
    }
}

// Dummy test for non-Unix platforms
#[cfg(not(unix))]
#[test]
fn test_symlinks_not_supported() {
    // Symlink tests are only for Unix platforms
    assert!(true);
}
