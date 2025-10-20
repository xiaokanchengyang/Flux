use flux_core::strategy::{Algorithm, CompressionStrategy};
use std::fs;
use tempfile::TempDir;

fn main() {
    // Enable debug logging
    env_logger::init();
    
    let temp_dir = TempDir::new().unwrap();
    
    // Test 1: Small text file
    let text_file = temp_dir.path().join("test.txt");
    fs::write(&text_file, "Some text content").unwrap();
    
    println!("\n=== Testing test.txt ===");
    println!("File size: {} bytes", fs::metadata(&text_file).unwrap().len());
    let strategy = CompressionStrategy::smart(&text_file, None, None);
    println!("Result: {:?}", strategy);
    println!("Expected: Zstd, Got: {:?}", strategy.algorithm);
    
    // Test 2: User override
    println!("\n=== Testing user override ===");
    let strategy_with_level = CompressionStrategy::smart(&text_file, Some(9), None);
    println!("With level 9: level = {}", strategy_with_level.level);
    
    // Test 3: High entropy file
    println!("\n=== Testing high entropy file ===");
    let random_file = temp_dir.path().join("random.bin");
    let random_data: Vec<u8> = (0..1024).map(|i| (i * 7 + 13) as u8).collect();
    fs::write(&random_file, random_data).unwrap();
    
    let strategy = CompressionStrategy::smart(&random_file, None, None);
    println!("Random file strategy: {:?}", strategy);
}