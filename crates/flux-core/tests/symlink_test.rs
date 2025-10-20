#[cfg(unix)]
mod symlink_tests {
    use flux_core::archive::{
        extract_with_options, pack_with_strategy, ExtractOptions, PackOptions,
    };
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_symlinks_follow() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test structure with symlink
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), "Target content").unwrap();
        unix_fs::symlink("target.txt", source_dir.join("link.txt")).unwrap();

        // Pack with follow_symlinks=true
        let pack_options = PackOptions {
            follow_symlinks: true,
            ..Default::default()
        };
        pack_with_strategy(&source_dir, &archive_path, Some("tar"), pack_options).unwrap();

        // Extract
        extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

        // Verify: link should be a regular file with target's content
        let link_path = extract_dir.join("link.txt");
        assert!(link_path.exists());
        assert!(!link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&link_path).unwrap(), "Target content");
    }

    #[test]
    fn test_pack_symlinks_preserve() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test structure with symlink
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), "Target content").unwrap();
        unix_fs::symlink("target.txt", source_dir.join("link.txt")).unwrap();

        // Pack with follow_symlinks=false (default)
        let pack_options = PackOptions {
            follow_symlinks: false,
            ..Default::default()
        };
        pack_with_strategy(&source_dir, &archive_path, Some("tar"), pack_options).unwrap();

        // Extract
        extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

        // Verify: link should still be a symlink
        let link_path = extract_dir.join("link.txt");
        assert!(link_path.exists());
        assert!(link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());

        // Verify link target
        let link_target = fs::read_link(&link_path).unwrap();
        assert_eq!(link_target.to_str().unwrap(), "target.txt");

        // Verify we can read through the symlink
        assert_eq!(fs::read_to_string(&link_path).unwrap(), "Target content");
    }

    #[test]
    fn test_pack_broken_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create broken symlink (target doesn't exist)
        fs::create_dir_all(&source_dir).unwrap();
        unix_fs::symlink("nonexistent.txt", source_dir.join("broken_link.txt")).unwrap();

        // Pack with follow_symlinks=false
        let pack_options = PackOptions {
            follow_symlinks: false,
            ..Default::default()
        };
        pack_with_strategy(&source_dir, &archive_path, Some("tar"), pack_options).unwrap();

        // Extract
        extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

        // Verify: broken link should be preserved
        let link_path = extract_dir.join("broken_link.txt");
        assert!(link_path.symlink_metadata().is_ok()); // Link exists
        assert!(link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(fs::read_link(&link_path).is_ok()); // Can read link target
        assert!(fs::read_to_string(&link_path).is_err()); // But can't read through it
    }

    #[test]
    fn test_pack_directory_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create directory and symlink to it
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(source_dir.join("real_dir")).unwrap();
        fs::write(source_dir.join("real_dir/file.txt"), "File in dir").unwrap();
        unix_fs::symlink("real_dir", source_dir.join("link_dir")).unwrap();

        // Pack with follow_symlinks=false
        pack_with_strategy(
            &source_dir,
            &archive_path,
            Some("tar"),
            PackOptions::default(),
        )
        .unwrap();

        // Extract
        extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

        // Verify
        let link_dir = extract_dir.join("link_dir");
        assert!(link_dir
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_link(&link_dir).unwrap().to_str().unwrap(),
            "real_dir"
        );
    }

    #[test]
    fn test_symlink_with_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create absolute symlink
        fs::create_dir_all(&source_dir).unwrap();
        let target_path = source_dir.join("target.txt");
        fs::write(&target_path, "Content").unwrap();

        // Create symlink with absolute path
        unix_fs::symlink(&target_path, source_dir.join("abs_link.txt")).unwrap();

        // Pack
        pack_with_strategy(
            &source_dir,
            &archive_path,
            Some("tar"),
            PackOptions::default(),
        )
        .unwrap();

        // Extract
        extract_with_options(&archive_path, &extract_dir, ExtractOptions::default()).unwrap();

        // The absolute symlink should be preserved as-is
        let link_path = extract_dir.join("abs_link.txt");
        assert!(link_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        let link_target = fs::read_link(&link_path).unwrap();
        assert_eq!(link_target, target_path);
    }
}
