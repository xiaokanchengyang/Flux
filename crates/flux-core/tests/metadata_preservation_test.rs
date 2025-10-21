//! Tests for metadata preservation during pack/extract

use flux_core::{extract_with_options, pack_with_strategy, ExtractOptions, PackOptions};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

#[test]
#[cfg(unix)]
#[ignore = "Metadata preservation not fully implemented"]
fn test_unix_permissions_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar.zst");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test files with different permissions
    fs::create_dir_all(&source_dir).unwrap();

    let executable = source_dir.join("executable.sh");
    fs::write(&executable, "#!/bin/bash\necho 'Hello'").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let readonly = source_dir.join("readonly.txt");
    fs::write(&readonly, "Read only file").unwrap();
    fs::set_permissions(&readonly, fs::Permissions::from_mode(0o444)).unwrap();

    let normal = source_dir.join("normal.txt");
    fs::write(&normal, "Normal file").unwrap();
    fs::set_permissions(&normal, fs::Permissions::from_mode(0o644)).unwrap();

    // Pack the files
    let options = PackOptions::default();
    pack_with_strategy(&source_dir, &archive_path, None, options).unwrap();

    // Extract the files
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: true,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify permissions were preserved
    let extracted_executable = extract_dir.join("source/executable.sh");
    let extracted_readonly = extract_dir.join("source/readonly.txt");
    let extracted_normal = extract_dir.join("source/normal.txt");

    assert_eq!(
        fs::metadata(&extracted_executable)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755,
        "Executable permissions not preserved"
    );

    assert_eq!(
        fs::metadata(&extracted_readonly)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o444,
        "Read-only permissions not preserved"
    );

    assert_eq!(
        fs::metadata(&extracted_normal)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644,
        "Normal file permissions not preserved"
    );
}

#[test]
fn test_modification_time_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.txt");
    let archive_path = temp_dir.path().join("test.tar.zst");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a file
    fs::write(&source_file, "Test content").unwrap();

    // Set a specific modification time (1 hour ago)
    let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
    filetime::set_file_mtime(
        &source_file,
        filetime::FileTime::from_system_time(one_hour_ago),
    )
    .unwrap();

    // Record the original mtime
    let original_mtime = fs::metadata(&source_file).unwrap().modified().unwrap();

    // Pack the file
    let options = PackOptions::default();
    pack_with_strategy(&source_file, &archive_path, None, options).unwrap();

    // Extract the file
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: true,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify modification time was preserved
    let extracted_file = extract_dir.join("test.txt");
    let extracted_mtime = fs::metadata(&extracted_file).unwrap().modified().unwrap();

    // Allow for some small difference due to timestamp precision
    let time_diff = if original_mtime > extracted_mtime {
        original_mtime.duration_since(extracted_mtime).unwrap()
    } else {
        extracted_mtime.duration_since(original_mtime).unwrap()
    };

    assert!(
        time_diff.as_secs() < 2,
        "Modification time not preserved accurately. Difference: {:?}",
        time_diff
    );
}

#[test]
#[cfg(unix)]
#[ignore = "Symlink preservation needs work"]
fn test_symlink_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar.zst");
    let extract_dir = temp_dir.path().join("extracted");

    // Create test structure with symlink
    fs::create_dir_all(&source_dir).unwrap();

    let target_file = source_dir.join("target.txt");
    fs::write(&target_file, "Target content").unwrap();

    let symlink = source_dir.join("link_to_target.txt");
    #[cfg(unix)]
    {
        use std::os::unix::fs;
        fs::symlink("target.txt", &symlink).unwrap();
    }

    // Pack without following symlinks
    let options = PackOptions {
        follow_symlinks: false,
        ..Default::default()
    };
    pack_with_strategy(&source_dir, &archive_path, None, options).unwrap();

    // Extract
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: true,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify symlink was preserved
    let extracted_symlink = extract_dir.join("source/link_to_target.txt");
    assert!(extracted_symlink
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());

    // Verify symlink target
    let link_target = fs::read_link(&extracted_symlink).unwrap();
    assert_eq!(link_target.to_str().unwrap(), "target.txt");
}

#[test]
#[ignore = "Directory structure preservation needs work"]
fn test_directory_structure_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("source");
    let archive_path = temp_dir.path().join("test.tar.zst");
    let extract_dir = temp_dir.path().join("extracted");

    // Create nested directory structure
    let deep_dir = source_dir.join("level1/level2/level3");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(deep_dir.join("deep.txt"), "Deep file").unwrap();
    fs::write(source_dir.join("root.txt"), "Root file").unwrap();
    fs::write(source_dir.join("level1/mid.txt"), "Mid file").unwrap();

    // Pack the directory
    let options = PackOptions::default();
    pack_with_strategy(&source_dir, &archive_path, None, options).unwrap();

    // Extract
    fs::create_dir_all(&extract_dir).unwrap();
    let extract_opts = ExtractOptions {
        overwrite: true,
        skip: false,
        rename: false,
        strip_components: None,
        hoist: true,
    };
    extract_with_options(&archive_path, &extract_dir, extract_opts).unwrap();

    // Verify directory structure
    assert!(extract_dir
        .join("source/level1/level2/level3/deep.txt")
        .exists());
    assert!(extract_dir.join("source/root.txt").exists());
    assert!(extract_dir.join("source/level1/mid.txt").exists());
}
