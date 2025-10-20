use flux_lib::strategy::{Algorithm, CompressionStrategy};
use std::fs;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let text_file = temp_dir.path().join("test.txt");
    fs::write(&text_file, "Some text content").unwrap();
    
    println!("File size: {} bytes", fs::metadata(&text_file).unwrap().len());
    
    let strategy = CompressionStrategy::smart(&text_file, None, None);
    println!("Strategy: {:?}", strategy);
    println!("Expected: Zstd, Got: {:?}", strategy.algorithm);
}
