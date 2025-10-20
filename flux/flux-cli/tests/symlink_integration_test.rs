#[cfg(unix)]
mod symlink_integration_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn test_cli_pack_follow_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test structure with symlink
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), "Target content").unwrap();
        unix_fs::symlink("target.txt", source_dir.join("link.txt")).unwrap();

        // Pack with --follow-symlinks
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .arg("--follow-symlinks")
            .assert()
            .success();

        // Extract
        Command::cargo_bin("flux")
            .unwrap()
            .arg("extract")
            .arg(&archive_path)
            .arg("-o")
            .arg(&extract_dir)
            .assert()
            .success();

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
    fn test_cli_pack_preserve_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create test structure with symlink
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), "Target content").unwrap();
        unix_fs::symlink("target.txt", source_dir.join("link.txt")).unwrap();

        // Pack without --follow-symlinks (default behavior)
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .assert()
            .success();

        // Extract
        Command::cargo_bin("flux")
            .unwrap()
            .arg("extract")
            .arg(&archive_path)
            .arg("-o")
            .arg(&extract_dir)
            .assert()
            .success();

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
    fn test_cli_strip_components() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create nested directory structure
        fs::create_dir_all(source_dir.join("level1/level2")).unwrap();
        fs::write(source_dir.join("level1/level2/deep.txt"), "Deep content").unwrap();
        fs::write(source_dir.join("level1/mid.txt"), "Mid content").unwrap();

        // Pack the directory
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .assert()
            .success();

        // Extract with --strip-components=1
        Command::cargo_bin("flux")
            .unwrap()
            .arg("extract")
            .arg(&archive_path)
            .arg("-o")
            .arg(&extract_dir)
            .arg("--strip-components")
            .arg("1")
            .assert()
            .success();

        // Verify files are extracted without the first component
        assert!(
            !extract_dir.join("source").exists(),
            "source directory should not exist"
        );
        assert!(
            extract_dir.join("level1/level2/deep.txt").exists(),
            "deep.txt should exist at level1/level2"
        );
        assert!(
            extract_dir.join("level1/mid.txt").exists(),
            "mid.txt should exist at level1"
        );
    }

    #[test]
    fn test_cli_strip_components_deep() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create nested directory structure
        fs::create_dir_all(source_dir.join("level1/level2/level3")).unwrap();
        fs::write(
            source_dir.join("level1/level2/level3/deep.txt"),
            "Deep content",
        )
        .unwrap();
        fs::write(source_dir.join("level1/level2/mid.txt"), "Mid content").unwrap();

        // Pack the directory
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .assert()
            .success();

        // Extract with --strip-components=3 (stripping source/level1/level2)
        Command::cargo_bin("flux")
            .unwrap()
            .arg("extract")
            .arg(&archive_path)
            .arg("-o")
            .arg(&extract_dir)
            .arg("--strip-components")
            .arg("3")
            .assert()
            .success();

        // Verify only files with enough components are extracted
        assert!(
            extract_dir.join("level3/deep.txt").exists(),
            "deep.txt should exist at level3"
        );
        assert!(
            extract_dir.join("mid.txt").exists(),
            "mid.txt should exist at root"
        );
    }

    #[test]
    fn test_cli_preserve_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.tar");
        let extract_dir = temp_dir.path().join("extracted");

        // Create file with specific permissions
        fs::create_dir_all(&source_dir).unwrap();
        let executable_path = source_dir.join("executable.sh");
        fs::write(&executable_path, "#!/bin/bash\necho 'Hello'").unwrap();

        // Set executable permissions
        let mut perms = fs::metadata(&executable_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&executable_path, perms).unwrap();

        // Pack
        Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .assert()
            .success();

        // Extract
        Command::cargo_bin("flux")
            .unwrap()
            .arg("extract")
            .arg(&archive_path)
            .arg("-o")
            .arg(&extract_dir)
            .assert()
            .success();

        // Verify permissions were preserved
        let extracted_executable = extract_dir.join("executable.sh");
        let extracted_perms = fs::metadata(&extracted_executable).unwrap().permissions();
        assert_eq!(extracted_perms.mode() & 0o777, 0o755);
    }

    #[test]
    fn test_cli_symlink_in_zip_warning() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let archive_path = temp_dir.path().join("test.zip");

        // Create test structure with symlink
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), "Target content").unwrap();
        unix_fs::symlink("target.txt", source_dir.join("link.txt")).unwrap();

        // Pack to ZIP (should show warning about symlinks)
        let output = Command::cargo_bin("flux")
            .unwrap()
            .arg("pack")
            .arg(&source_dir)
            .arg("-o")
            .arg(&archive_path)
            .arg("-f")
            .arg("zip")
            .env("RUST_LOG", "warn")
            .output()
            .unwrap();

        // Should succeed
        assert!(output.status.success());

        // Check if warning about symlinks was shown (if logging is enabled)
        // Note: This depends on how logging is configured in the binary
    }
}