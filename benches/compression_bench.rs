//! Compression performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use flux_core::archive::{pack_with_strategy, PackOptions};
use flux_core::strategy::Algorithm;
use rand::{Rng, SeedableRng};
use std::fs::{self, File};
use std::io::Write;
// Remove unused import flagged by clippy
use tempfile::TempDir;

/// Generate test data with specified characteristics
fn generate_test_data(dir: &TempDir, file_count: usize, file_size: usize, compressible: bool) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    for i in 0..file_count {
        let file_path = dir.path().join(format!("file_{}.dat", i));
        let mut file = File::create(file_path).unwrap();

        if compressible {
            // Generate compressible data (repeated patterns)
            let pattern = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
            let repetitions = file_size / pattern.len();
            for _ in 0..repetitions {
                file.write_all(pattern).unwrap();
            }
        } else {
            // Generate random data (incompressible)
            let mut data = vec![0u8; file_size];
            rng.fill(&mut data[..]);
            file.write_all(&data).unwrap();
        }
    }
}

/// Benchmark packing many small files
fn bench_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_files");
    group.sample_size(10);

    let algorithms = vec![
        ("zstd", Algorithm::Zstd),
        ("gzip", Algorithm::Gzip),
        ("xz", Algorithm::Xz),
        ("store", Algorithm::Store),
    ];

    for (name, algorithm) in algorithms {
        group.bench_with_input(
            BenchmarkId::new("pack_1000_small_files", name),
            &algorithm,
            |b, &algorithm| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        generate_test_data(&temp_dir, 1000, 1024, true); // 1000 files, 1KB each
                        (temp_dir, TempDir::new().unwrap())
                    },
                    |(input_dir, output_dir)| {
                        let output = output_dir.path().join("archive.tar.zst");
                        let options = PackOptions {
                            smart: false,
                            algorithm: Some(algorithm.to_string()),
                            level: Some(3),
                            threads: Some(4),
                            force_compress: false,
                            follow_symlinks: false,
                        };

                        pack_with_strategy(
                            black_box(input_dir.path()),
                            black_box(&output),
                            None,
                            options,
                        )
                        .unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark packing single large file
fn bench_large_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file");
    group.sample_size(10);

    let algorithms = vec![
        ("zstd", Algorithm::Zstd),
        ("gzip", Algorithm::Gzip),
        ("xz", Algorithm::Xz),
    ];

    for (name, algorithm) in algorithms {
        group.bench_with_input(
            BenchmarkId::new("pack_100mb_file", name),
            &algorithm,
            |b, &algorithm| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        generate_test_data(&temp_dir, 1, 100 * 1024 * 1024, true); // 1 file, 100MB
                        (temp_dir, TempDir::new().unwrap())
                    },
                    |(input_dir, output_dir)| {
                        let output = output_dir.path().join("archive.tar.zst");
                        let options = PackOptions {
                            smart: false,
                            algorithm: Some(algorithm.to_string()),
                            level: Some(3),
                            threads: Some(4),
                            force_compress: false,
                            follow_symlinks: false,
                        };

                        pack_with_strategy(
                            black_box(input_dir.path()),
                            black_box(&output),
                            None,
                            options,
                        )
                        .unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark compression algorithms with different levels
fn bench_compression_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_levels");
    group.sample_size(10);

    let levels = vec![1, 3, 6, 9];

    for level in levels {
        group.bench_with_input(BenchmarkId::new("zstd_10mb", level), &level, |b, &level| {
            b.iter_with_setup(
                || {
                    let temp_dir = TempDir::new().unwrap();
                    generate_test_data(&temp_dir, 1, 10 * 1024 * 1024, true); // 10MB file
                    (temp_dir, TempDir::new().unwrap())
                },
                |(input_dir, output_dir)| {
                    let output = output_dir.path().join("archive.tar.zst");
                    let options = PackOptions {
                        smart: false,
                        algorithm: Some("zstd".to_string()),
                        level: Some(level),
                        threads: Some(4),
                        force_compress: false,
                        follow_symlinks: false,
                    };

                    pack_with_strategy(
                        black_box(input_dir.path()),
                        black_box(&output),
                        None,
                        options,
                    )
                    .unwrap();
                },
            );
        });
    }

    group.finish();
}

/// Benchmark smart strategy vs manual selection
fn bench_smart_strategy(c: &mut Criterion) {
    let mut group = c.benchmark_group("smart_strategy");
    group.sample_size(10);

    // Test with mixed content
    group.bench_function("smart_mixed_content", |b| {
        b.iter_with_setup(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create mixed content: text files, images, and binary
                for i in 0..10 {
                    let text_file = temp_dir.path().join(format!("doc_{}.txt", i));
                    fs::write(&text_file, "This is a text document.\n".repeat(1000)).unwrap();
                }
                for i in 0..5 {
                    let _binary_file = temp_dir.path().join(format!("data_{}.bin", i));
                    generate_test_data(&temp_dir, 1, 1024 * 1024, false);
                }
                (temp_dir, TempDir::new().unwrap())
            },
            |(input_dir, output_dir)| {
                let output = output_dir.path().join("archive.tar");
                let options = PackOptions {
                    smart: true,
                    algorithm: None,
                    level: None,
                    threads: None,
                    force_compress: false,
                    follow_symlinks: false,
                };

                pack_with_strategy(
                    black_box(input_dir.path()),
                    black_box(&output),
                    None,
                    options,
                )
                .unwrap();
            },
        );
    });

    group.bench_function("manual_zstd_mixed_content", |b| {
        b.iter_with_setup(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Same mixed content as above
                for i in 0..10 {
                    let text_file = temp_dir.path().join(format!("doc_{}.txt", i));
                    fs::write(&text_file, "This is a text document.\n".repeat(1000)).unwrap();
                }
                for i in 0..5 {
                    let _binary_file = temp_dir.path().join(format!("data_{}.bin", i));
                    generate_test_data(&temp_dir, 1, 1024 * 1024, false);
                }
                (temp_dir, TempDir::new().unwrap())
            },
            |(input_dir, output_dir)| {
                let output = output_dir.path().join("archive.tar.zst");
                let options = PackOptions {
                    smart: false,
                    algorithm: Some("zstd".to_string()),
                    level: Some(3),
                    threads: Some(4),
                    force_compress: false,
                    follow_symlinks: false,
                };

                pack_with_strategy(
                    black_box(input_dir.path()),
                    black_box(&output),
                    None,
                    options,
                )
                .unwrap();
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_files,
    bench_large_file,
    bench_compression_levels,
    bench_smart_strategy
);
criterion_main!(benches);
