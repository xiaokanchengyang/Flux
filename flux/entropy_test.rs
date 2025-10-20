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
    let text = "Some text content";
    let entropy = calculate_entropy(text.as_bytes());
    println!("Text '{}' entropy: {:.2}", text, entropy);
    println!("Is it high entropy (> 7.5)? {}", entropy > 7.5);
}
