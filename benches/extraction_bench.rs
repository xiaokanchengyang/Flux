//! Extraction performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use flux_core::archive::{extract, pack_with_strategy, PackOptions};
use flux_core::strategy::Algorithm;
use rand::SeedableRng;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Create a test archive with specified characteristics
fn create_test_archive(
    archive_path: &Path,
    file_count: usize,
    file_size: usize,
    algorithm: Algorithm,
    level: u32,
) {
    let temp_dir = TempDir::new().unwrap();
    let _rng = rand::rngs::StdRng::seed_from_u64(42);

    // Generate test files
    for i in 0..file_count {
        let file_path = temp_dir.path().join(format!("file_{}.dat", i));
        let mut file = File::create(file_path).unwrap();

        // Generate compressible data
        let pattern = format!("Test data for file {} - Lorem ipsum dolor sit amet. ", i);
        let data = pattern.repeat(file_size / pattern.len());
        file.write_all(data.as_bytes()).unwrap();
    }

    // Pack into archive
    let options = PackOptions {
        smart: false,
        algorithm: Some(algorithm.to_string()),
        level: Some(level),
        threads: Some(4),
        force_compress: false,
        follow_symlinks: false,
    };

    pack_with_strategy(temp_dir.path(), archive_path, None, options).unwrap();
}

/// Benchmark extracting archives with different compression algorithms
fn bench_extract_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_algorithms");
    group.sample_size(10);

    let algorithms = vec![
        ("tar", Algorithm::Store),
        ("tar.gz", Algorithm::Gzip),
        ("tar.zst", Algorithm::Zstd),
        ("tar.xz", Algorithm::Xz),
    ];

    for (ext, algorithm) in algorithms {
        group.bench_with_input(
            BenchmarkId::new("extract_100_files", ext),
            &algorithm,
            |b, &algorithm| {
                b.iter_with_setup(
                    || {
                        let archive_dir = TempDir::new().unwrap();
                        let archive_path = archive_dir.path().join(format!("test.{}", ext));
                        create_test_archive(&archive_path, 100, 10240, algorithm, 3); // 100 files, 10KB each
                        (archive_dir, archive_path)
                    },
                    |(_archive_dir, archive_path)| {
                        let output_dir = TempDir::new().unwrap();
                        extract(black_box(&archive_path), black_box(output_dir.path())).unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark extracting large archives
fn bench_extract_large_archive(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_large_archive");
    group.sample_size(5);

    group.bench_function("extract_1000_files_zstd", |b| {
        b.iter_with_setup(
            || {
                let archive_dir = TempDir::new().unwrap();
                let archive_path = archive_dir.path().join("large.tar.zst");
                create_test_archive(&archive_path, 1000, 10240, Algorithm::Zstd, 3); // 1000 files
                (archive_dir, archive_path)
            },
            |(_archive_dir, archive_path)| {
                let output_dir = TempDir::new().unwrap();
                extract(black_box(&archive_path), black_box(output_dir.path())).unwrap();
            },
        );
    });

    group.finish();
}

/// Benchmark parallel vs single-threaded extraction (if applicable)
fn bench_extract_parallelism(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_parallelism");
    group.sample_size(5);

    // This benchmark would require implementing parallel extraction support
    // For now, we'll just benchmark the current implementation
    group.bench_function("extract_zstd_current", |b| {
        b.iter_with_setup(
            || {
                let archive_dir = TempDir::new().unwrap();
                let archive_path = archive_dir.path().join("test.tar.zst");
                create_test_archive(&archive_path, 500, 20480, Algorithm::Zstd, 3);
                (archive_dir, archive_path)
            },
            |(_archive_dir, archive_path)| {
                let output_dir = TempDir::new().unwrap();
                extract(black_box(&archive_path), black_box(output_dir.path())).unwrap();
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_algorithms,
    bench_extract_large_archive,
    bench_extract_parallelism
);
criterion_main!(benches);
