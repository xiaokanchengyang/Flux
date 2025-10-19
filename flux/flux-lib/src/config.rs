//! Configuration module

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs::config_dir;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default compression settings
    pub compression: CompressionConfig,
    /// Archive format preferences
    pub archive: ArchiveConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
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
                threads: 0, // Auto-detect
                memory_limit: 0, // Unlimited
                buffer_size: 64, // 64KB
            },
        }
    }
}

impl Config {
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = config_dir()
            .ok_or_else(|| Error::ConfigError("Unable to determine config directory".to_string()))?;
        
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
    use tempfile::TempDir;

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
        
        assert_eq!(config.compression.default_algorithm, deserialized.compression.default_algorithm);
        assert_eq!(config.performance.threads, deserialized.performance.threads);
    }
}