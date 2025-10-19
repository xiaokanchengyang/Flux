use flux_lib::strategy::{Algorithm, CompressionStrategy};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_smart_strategy_for_text_files() {
    let temp_dir = TempDir::new().unwrap();

    // Test various text file extensions
    let text_files = vec![
        ("test.txt", Algorithm::Zstd),
        ("test.log", Algorithm::Zstd),
        ("test.json", Algorithm::Zstd),
        ("test.xml", Algorithm::Zstd),
        ("test.py", Algorithm::Zstd),
        ("test.rs", Algorithm::Zstd),
        ("test.js", Algorithm::Zstd),
    ];

    for (filename, expected_algo) in text_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, "Some text content").unwrap();

        let strategy = CompressionStrategy::smart(&file_path, None, None);
        assert_eq!(strategy.algorithm, expected_algo, "Failed for {}", filename);
        assert!(
            strategy.level > 3,
            "Text files should have higher compression level"
        );
    }
}

#[test]
fn test_smart_strategy_for_compressed_files() {
    let temp_dir = TempDir::new().unwrap();

    // Test various compressed file extensions
    let compressed_files = vec![
        "test.jpg", "test.png", "test.zip", "test.mp4", "test.mp3", "test.pdf", "test.7z",
        "test.gz",
    ];

    for filename in compressed_files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, "Fake compressed data").unwrap();

        let strategy = CompressionStrategy::smart(&file_path, None, None);
        assert_eq!(
            strategy.algorithm,
            Algorithm::Store,
            "Compressed file {} should use Store algorithm",
            filename
        );
    }
}

#[test]
fn test_smart_strategy_for_large_files() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.bin");

    // Create a large file (101MB)
    let large_data = vec![0u8; 101 * 1024 * 1024];
    fs::write(&large_file, large_data).unwrap();

    let strategy = CompressionStrategy::smart(&large_file, None, None);
    assert_eq!(strategy.algorithm, Algorithm::Xz);
    assert_eq!(
        strategy.threads, 1,
        "XZ should use single thread for memory efficiency"
    );
}

#[test]
fn test_smart_strategy_with_user_overrides() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.txt");
    fs::write(&file, "content").unwrap();

    // Test level override
    let strategy = CompressionStrategy::smart(&file, Some(9), None);
    assert_eq!(strategy.level, 9);

    // Test threads override
    let strategy = CompressionStrategy::smart(&file, None, Some(2));
    assert_eq!(strategy.threads, 2);
}

#[test]
fn test_smart_strategy_for_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create mixed content directory
    fs::write(temp_dir.path().join("file1.txt"), "text content").unwrap();
    fs::write(temp_dir.path().join("file2.log"), "log content").unwrap();
    fs::write(temp_dir.path().join("file3.rs"), "rust code").unwrap();
    fs::write(temp_dir.path().join("image.jpg"), "fake jpeg").unwrap();
    fs::write(temp_dir.path().join("video.mp4"), "fake video").unwrap();

    // Test directory with mostly text files
    let strategy = CompressionStrategy::smart_for_directory(temp_dir.path(), None, None).unwrap();
    assert_eq!(strategy.algorithm, Algorithm::Zstd);
    assert!(strategy.level >= 3);
}

#[test]
fn test_smart_strategy_for_compressed_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create directory with mostly compressed files
    fs::write(temp_dir.path().join("img1.jpg"), "fake jpeg").unwrap();
    fs::write(temp_dir.path().join("img2.png"), "fake png").unwrap();
    fs::write(temp_dir.path().join("video.mp4"), "fake video").unwrap();
    fs::write(temp_dir.path().join("archive.zip"), "fake zip").unwrap();
    fs::write(temp_dir.path().join("small.txt"), "text").unwrap();

    let strategy = CompressionStrategy::smart_for_directory(temp_dir.path(), None, None).unwrap();
    assert_eq!(strategy.algorithm, Algorithm::Store);
}

#[test]
fn test_entropy_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with high entropy (random data)
    let random_file = temp_dir.path().join("random.bin");
    let random_data: Vec<u8> = (0..1024).map(|i| (i * 7 + 13) as u8).collect();
    fs::write(&random_file, random_data).unwrap();

    // Even without compressed extension, high entropy should be detected
    let strategy = CompressionStrategy::smart(&random_file, None, None);
    assert_eq!(
        strategy.algorithm,
        Algorithm::Store,
        "High entropy file should use Store algorithm"
    );
}

#[test]
fn test_adjust_for_parallel() {
    let mut strategy = CompressionStrategy::default();

    // Test Zstd - should get max threads
    strategy.algorithm = Algorithm::Zstd;
    strategy.adjust_for_parallel();
    assert!(strategy.threads >= 4);

    // Test XZ - should be limited
    strategy.algorithm = Algorithm::Xz;
    strategy.threads = 8;
    strategy.adjust_for_parallel();
    assert!(strategy.threads <= 2);

    // Test Store - should be 1
    strategy.algorithm = Algorithm::Store;
    strategy.threads = 8;
    strategy.adjust_for_parallel();
    assert_eq!(strategy.threads, 1);
}
