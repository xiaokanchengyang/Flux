//! Configuration module

use crate::{Error, Result};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default compression settings
    pub compression: CompressionConfig,
    /// Archive format preferences
    pub archive: ArchiveConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// Strategy settings
    #[serde(default)]
    pub strategy: StrategyConfig,
    /// Custom compression rules
    #[serde(default)]
    pub rules: Vec<CompressionRule>,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Default compression algorithm
    pub default_algorithm: String,
    /// Default compression level (1-9)
    pub default_level: u32,
    /// Force compression on already compressed files
    pub force_compress: bool,
    /// Enable smart compression strategy
    pub smart_strategy: bool,
}

/// Archive format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// Default archive format
    pub default_format: String,
    /// Preserve metadata by default
    pub preserve_metadata: bool,
    /// Follow symlinks by default
    pub follow_symlinks: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of threads to use (0 = auto)
    pub threads: usize,
    /// Memory limit in MB (0 = unlimited)
    pub memory_limit: u64,
    /// Buffer size in KB
    pub buffer_size: u32,
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Large file threshold in bytes
    pub large_file_threshold: Option<u64>,
    /// Enable zstd long mode for very large files
    pub enable_long_mode: bool,
    /// Memory limit for compression (in MB)
    pub memory_limit: Option<u32>,
    /// Size-based compression rules
    #[serde(default, deserialize_with = "deserialize_size_rules")]
    pub size_rules: Vec<SizeRule>,
}

/// Size rule with string threshold support
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SizeRuleConfig {
    /// Size rule with numeric threshold
    Numeric(SizeRule),
    /// Size rule with string threshold
    String {
        threshold: String,
        algorithm: String,
        level: u32,
    },
}

/// Deserialize size rules from either numeric or string format
fn deserialize_size_rules<'de, D>(deserializer: D) -> std::result::Result<Vec<SizeRule>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let configs: Vec<SizeRuleConfig> = Vec::deserialize(deserializer)?;
    configs
        .into_iter()
        .map(|config| match config {
            SizeRuleConfig::Numeric(rule) => Ok(rule),
            SizeRuleConfig::String {
                threshold,
                algorithm,
                level,
            } => parse_size(&threshold)
                .map(|threshold_bytes| SizeRule {
                    threshold: threshold_bytes,
                    algorithm,
                    level,
                })
                .map_err(|e| D::Error::custom(format!("Failed to parse threshold: {}", e))),
        })
        .collect()
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            large_file_threshold: None,
            enable_long_mode: true,
            memory_limit: None,
            size_rules: vec![
                // Example: Use XZ for files over 128 MiB
                // You can also use string format in config file like:
                // [[strategy.size_rules]]
                // threshold = "128MiB"
                // algorithm = "xz"
                // level = 7
                SizeRule {
                    threshold: 128 * 1024 * 1024, // 128 MiB
                    algorithm: "xz".to_string(),
                    level: 7,
                },
            ],
        }
    }
}

/// Size-based compression rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRule {
    /// File size threshold in bytes
    pub threshold: u64,
    /// Algorithm to use for files above this threshold
    pub algorithm: String,
    /// Compression level
    pub level: u32,
}

/// Parse size string like "100MiB" to bytes
pub fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim();

    // Try to parse as plain number first
    if let Ok(bytes) = size_str.parse::<u64>() {
        return Ok(bytes);
    }

    // Find where the number ends and unit begins
    let split_pos = size_str
        .chars()
        .position(|c| !c.is_ascii_digit() && c != '.')
        .unwrap_or(size_str.len());

    if split_pos == 0 {
        return Err(Error::Config(format!(
            "Invalid size format: {}",
            size_str
        )));
    }

    let (number_part, unit_part) = size_str.split_at(split_pos);
    let number: f64 = number_part
        .parse()
        .map_err(|_| Error::Config(format!("Invalid number in size: {}", number_part)))?;

    let multiplier: i64 = match unit_part.trim().to_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1_000,
        "m" | "mb" => 1_000_000,
        "g" | "gb" => 1_000_000_000,
        "t" | "tb" => 1_000_000_000_000,
        "ki" | "kib" => 1_024,
        "mi" | "mib" => 1_048_576,
        "gi" | "gib" => 1_073_741_824,
        "ti" | "tib" => 1_099_511_627_776,
        _ => {
            return Err(Error::Config(format!(
                "Unknown size unit: {}",
                unit_part
            )))
        }
    };

    Ok((number * multiplier as f64) as u64)
}

/// Custom compression rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionRule {
    /// Rule name
    pub name: String,
    /// File patterns to match (glob patterns)
    pub patterns: Vec<String>,
    /// Minimum file size in bytes (optional)
    pub min_size: Option<u64>,
    /// Maximum file size in bytes (optional)
    pub max_size: Option<u64>,
    /// Compression algorithm to use
    pub algorithm: String,
    /// Compression level (optional)
    pub level: Option<u32>,
    /// Number of threads (optional)
    pub threads: Option<usize>,
    /// Priority (higher priority rules are evaluated first)
    pub priority: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            compression: CompressionConfig {
                default_algorithm: "zstd".to_string(),
                default_level: 3,
                force_compress: false,
                smart_strategy: true,
            },
            archive: ArchiveConfig {
                default_format: "tar.zst".to_string(),
                preserve_metadata: true,
                follow_symlinks: false,
            },
            performance: PerformanceConfig {
                threads: 0,      // Auto-detect
                memory_limit: 0, // Unlimited
                buffer_size: 64, // 64KB
            },
            strategy: StrategyConfig::default(),
            rules: vec![
                // Example rule: Use brotli for HTML/CSS/JS files
                CompressionRule {
                    name: "web_assets".to_string(),
                    patterns: vec![
                        "*.html".to_string(),
                        "*.css".to_string(),
                        "*.js".to_string(),
                    ],
                    min_size: None,
                    max_size: None,
                    algorithm: "brotli".to_string(),
                    level: Some(11),
                    threads: None,
                    priority: 100,
                },
                // Example rule: Store very small files without compression
                CompressionRule {
                    name: "tiny_files".to_string(),
                    patterns: vec!["*".to_string()],
                    min_size: None,
                    max_size: Some(10), // Less than 10 bytes
                    algorithm: "store".to_string(),
                    level: None,
                    threads: None,
                    priority: 90,
                },
                // Example rule: Use XZ for large archive files
                CompressionRule {
                    name: "large_archives".to_string(),
                    patterns: vec!["*.tar".to_string(), "*.iso".to_string()],
                    min_size: Some(100 * 1024 * 1024), // > 100MB
                    max_size: None,
                    algorithm: "xz".to_string(),
                    level: Some(6),
                    threads: Some(1),
                    priority: 95,
                },
            ],
        }
    }
}

impl Config {
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = config_dir().ok_or_else(|| {
            Error::Config("Unable to determine config directory".to_string())
        })?;

        let flux_dir = config_dir.join("flux");
        if !flux_dir.exists() {
            fs::create_dir_all(&flux_dir)?;
        }

        Ok(flux_dir.join("config.toml"))
    }

    /// Get default configuration content with examples
    pub fn default_config_content() -> String {
        r#"# Flux Configuration File
# This file configures the behavior of the flux archiver

[compression]
# Default compression algorithm: zstd, xz, brotli, gzip, store
default_algorithm = "zstd"
# Default compression level (1-9 for most algorithms)
default_level = 3
# Force compression on already compressed files
force_compress = false
# Enable smart compression strategy
smart_strategy = true

[archive]
# Default archive format: tar.zst, tar.gz, tar.xz, tar.br, tar, zip
default_format = "tar.zst"
# Preserve file metadata (permissions, timestamps)
preserve_metadata = true
# Follow symlinks when archiving
follow_symlinks = false

[performance]
# Number of threads to use (0 = auto-detect)
threads = 0
# Memory limit in MB (0 = unlimited)
memory_limit = 0
# Buffer size in KB
buffer_size = 64

[strategy]
# Large file threshold in bytes (files above this use memory-efficient settings)
# large_file_threshold = 104857600  # 100 MiB

# Enable zstd long mode for very large files
enable_long_mode = true

# Memory limit for compression in MB
# memory_limit = 512

# Size-based compression rules
# Rules are evaluated in order, first matching rule wins
[[strategy.size_rules]]
# Use XZ with high compression for files over 128 MiB
threshold = "128MiB"
algorithm = "xz"
level = 7

# [[strategy.size_rules]]
# # Use fast Zstd for files over 1 GiB
# threshold = "1GiB"
# algorithm = "zstd"
# level = 1

# Custom compression rules based on file patterns
[[rules]]
name = "web_assets"
patterns = ["*.html", "*.css", "*.js"]
algorithm = "brotli"
level = 11
priority = 100

[[rules]]
name = "tiny_files"
patterns = ["*"]
max_size = 10  # Less than 10 bytes
algorithm = "store"
priority = 90

[[rules]]
name = "large_archives"
patterns = ["*.tar", "*.iso"]
min_size = 104857600  # > 100MB
algorithm = "xz"
level = 6
threads = 1
priority = 95
"#
        .to_string()
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            // Create default config with detailed examples
            let default_content = Self::default_config_content();
            fs::write(&path, default_content)?;
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&path, contents)?;
        Ok(())
    }

    /// Load configuration or use defaults if loading fails
    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.compression.default_algorithm, "zstd");
        assert_eq!(config.compression.default_level, 3);
        assert!(config.compression.smart_strategy);
        assert!(!config.compression.force_compress);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            config.compression.default_algorithm,
            deserialized.compression.default_algorithm
        );
        assert_eq!(config.performance.threads, deserialized.performance.threads);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1_000);
        assert_eq!(parse_size("1KiB").unwrap(), 1_024);
        assert_eq!(parse_size("100MiB").unwrap(), 100 * 1_048_576);
        assert_eq!(parse_size("1.5GB").unwrap(), 1_500_000_000);
        assert_eq!(parse_size("2GiB").unwrap(), 2 * 1_073_741_824);
        assert!(parse_size("invalid").is_err());
    }

    #[test]
    fn test_size_rules_deserialization() {
        let toml_str = r#"
            enable_long_mode = true
            
            [[size_rules]]
            threshold = "100MiB"
            algorithm = "xz"
            level = 7
            
            [[size_rules]]
            threshold = 52428800
            algorithm = "zstd"
            level = 3
        "#;

        let config: StrategyConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.size_rules.len(), 2);
        assert_eq!(config.size_rules[0].threshold, 100 * 1_048_576);
        assert_eq!(config.size_rules[0].algorithm, "xz");
        assert_eq!(config.size_rules[1].threshold, 52428800);
        assert_eq!(config.size_rules[1].algorithm, "zstd");
    }
}
