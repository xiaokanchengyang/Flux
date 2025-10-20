//! Comparison benchmarks with system tar command

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flux_core::archive::{pack_with_strategy, PackOptions};
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Generate test directory with realistic content
fn generate_realistic_content(dir: &TempDir) {
    // Create a mix of files that resembles a typical project
    
    // Source code files
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    
    for i in 0..20 {
        let file_path = src_dir.join(format!("module_{}.rs", i));
        let content = format!(
            r#"// Module {}
use std::collections::HashMap;

pub struct Module{} {{
    data: HashMap<String, String>,
}}

impl Module{} {{
    pub fn new() -> Self {{
        Self {{
            data: HashMap::new(),
        }}
    }}
    
    pub fn process(&mut self, input: &str) -> String {{
        // Some processing logic here
        input.to_uppercase()
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_module() {{
        let mut m = Module{}::new();
        assert_eq!(m.process("hello"), "HELLO");
    }}
}}
"#,
            i, i, i, i
        );
        fs::write(file_path, content).unwrap();
    }
    
    // Documentation files
    let docs_dir = dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    
    for i in 0..10 {
        let file_path = docs_dir.join(format!("chapter_{}.md", i));
        let content = format!(
            "# Chapter {}\n\nThis is documentation for chapter {}.\n\n{}",
            i,
            i,
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(50)
        );
        fs::write(file_path, content).unwrap();
    }
    
    // Configuration files
    let config_content = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#;
    fs::write(dir.path().join("Cargo.toml"), config_content).unwrap();
    
    // Binary data
    let data_dir = dir.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    
    for i in 0..5 {
        let file_path = data_dir.join(format!("data_{}.bin", i));
        let data = vec![i as u8; 1024 * 100]; // 100KB of repeated bytes
        fs::write(file_path, data).unwrap();
    }
}

/// Benchmark Flux vs system tar for creating archives
fn bench_flux_vs_tar(c: &mut Criterion) {
    let mut group = c.benchmark_group("flux_vs_tar");
    group.sample_size(10);
    
    // Benchmark Flux
    group.bench_function("flux_pack_project", |b| {
        b.iter_with_setup(
            || {
                let input_dir = TempDir::new().unwrap();
                generate_realistic_content(&input_dir);
                (input_dir, TempDir::new().unwrap())
            },
            |(input_dir, output_dir)| {
                let output = output_dir.path().join("archive.tar.zst");
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
                ).unwrap();
            },
        );
    });
    
    // Benchmark system tar (if available)
    if Command::new("tar").arg("--version").output().is_ok() {
        group.bench_function("system_tar_pack_project", |b| {
            b.iter_with_setup(
                || {
                    let input_dir = TempDir::new().unwrap();
                    generate_realistic_content(&input_dir);
                    (input_dir, TempDir::new().unwrap())
                },
                |(input_dir, output_dir)| {
                    let output = output_dir.path().join("archive.tar.gz");
                    
                    Command::new("tar")
                        .arg("-czf")
                        .arg(&output)
                        .arg("-C")
                        .arg(input_dir.path())
                        .arg(".")
                        .output()
                        .unwrap();
                },
            );
        });
    }
    
    group.finish();
}

/// Benchmark compression ratio comparison
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");
    group.sample_size(5);
    
    // This is more of a measurement than a benchmark
    // We'll create archives and compare sizes
    
    group.bench_function("measure_flux_compression", |b| {
        b.iter_with_setup(
            || {
                let input_dir = TempDir::new().unwrap();
                generate_realistic_content(&input_dir);
                let output_dir = TempDir::new().unwrap();
                (input_dir, output_dir)
            },
            |(input_dir, output_dir)| {
                let output = output_dir.path().join("archive.tar.zst");
                let options = PackOptions {
                    smart: true,
                    algorithm: Some("zstd".to_string()),
                    level: Some(3),
                    threads: None,
                    force_compress: false,
                    follow_symlinks: false,
                };
                
                pack_with_strategy(
                    input_dir.path(),
                    &output,
                    None,
                    options,
                ).unwrap();
                
                // Measure compression ratio
                let input_size: u64 = walkdir::WalkDir::new(input_dir.path())
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                    .sum();
                    
                let output_size = fs::metadata(&output).unwrap().len();
                let ratio = input_size as f64 / output_size as f64;
                
                black_box(ratio);
            },
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_flux_vs_tar,
    bench_compression_ratio
);
criterion_main!(benches);