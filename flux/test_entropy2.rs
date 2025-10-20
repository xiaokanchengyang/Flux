fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    // Count byte frequencies
    let mut freq = [0u64; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    // Calculate entropy
    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

fn main() {
    let random_data: Vec<u8> = (0..1024).map(|i| (i * 7 + 13) as u8).collect();
    let entropy = calculate_entropy(&random_data);
    println!("Test data entropy: {:.2}", entropy);
    println!("Is it high entropy (> 7.5)? {}", entropy > 7.5);
    
    // Check actual values
    let unique_values: std::collections::HashSet<_> = random_data.iter().collect();
    println!("Unique values: {}", unique_values.len());
}
