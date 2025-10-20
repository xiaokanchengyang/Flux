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
    #[serde(default)]
    pub size_rules: Vec<SizeRule>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            large_file_threshold: None,
            enable_long_mode: true,
            memory_limit: None,
            size_rules: vec![
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
            Error::ConfigError("Unable to determine config directory".to_string())
        })?;

        let flux_dir = config_dir.join("flux");
        if !flux_dir.exists() {
            fs::create_dir_all(&flux_dir)?;
        }

        Ok(flux_dir.join("config.toml"))
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            // Create default config if it doesn't exist
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize config: {}", e)))?;

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
}
