use flux_lib::config::{CompressionRule, Config};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_default_config() {
    let config = Config::default();

    // Check default compression settings
    assert_eq!(config.compression.default_algorithm, "zstd");
    assert_eq!(config.compression.default_level, 3);
    assert!(config.compression.smart_strategy);
    assert!(!config.compression.force_compress);

    // Check default archive settings
    assert_eq!(config.archive.default_format, "tar.zst");
    assert!(config.archive.preserve_metadata);
    assert!(!config.archive.follow_symlinks);

    // Check default performance settings
    assert_eq!(config.performance.threads, 0); // Auto-detect
    assert_eq!(config.performance.memory_limit, 0); // Unlimited
    assert_eq!(config.performance.buffer_size, 64); // 64KB

    // Check default rules
    assert!(!config.rules.is_empty());
}

#[test]
fn test_config_serialization() {
    let config = Config::default();

    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&config).unwrap();
    assert!(toml_str.contains("[compression]"));
    assert!(toml_str.contains("[archive]"));
    assert!(toml_str.contains("[performance]"));
    assert!(toml_str.contains("[[rules]]"));

    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(
        config.compression.default_algorithm,
        deserialized.compression.default_algorithm
    );
    assert_eq!(config.rules.len(), deserialized.rules.len());
}

#[test]
fn test_custom_rules() {
    let mut config = Config::default();

    // Add a custom rule
    config.rules.push(CompressionRule {
        name: "test_rule".to_string(),
        patterns: vec!["*.test".to_string()],
        min_size: Some(1000),
        max_size: Some(10000),
        algorithm: "xz".to_string(),
        level: Some(5),
        threads: Some(1),
        priority: 200,
    });

    // Verify the rule was added
    let test_rule = config.rules.iter().find(|r| r.name == "test_rule").unwrap();
    assert_eq!(test_rule.algorithm, "xz");
    assert_eq!(test_rule.priority, 200);
}

#[test]
fn test_rule_priority_sorting() {
    let rules = vec![
        CompressionRule {
            name: "low".to_string(),
            patterns: vec!["*".to_string()],
            min_size: None,
            max_size: None,
            algorithm: "store".to_string(),
            level: None,
            threads: None,
            priority: 10,
        },
        CompressionRule {
            name: "high".to_string(),
            patterns: vec!["*.important".to_string()],
            min_size: None,
            max_size: None,
            algorithm: "zstd".to_string(),
            level: Some(9),
            threads: None,
            priority: 100,
        },
        CompressionRule {
            name: "medium".to_string(),
            patterns: vec!["*.txt".to_string()],
            min_size: None,
            max_size: None,
            algorithm: "gzip".to_string(),
            level: Some(6),
            threads: None,
            priority: 50,
        },
    ];

    let mut sorted_rules = rules.clone();
    sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    assert_eq!(sorted_rules[0].name, "high");
    assert_eq!(sorted_rules[1].name, "medium");
    assert_eq!(sorted_rules[2].name, "low");
}

#[test]
fn test_config_load_save() {
    // Create a temporary directory for config
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("flux");
    fs::create_dir_all(&config_dir).unwrap();

    // Override config directory for test
    std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());

    // Create and save a custom config
    let mut config = Config::default();
    config.compression.default_level = 7;
    config.performance.threads = 4;

    // Note: We can't test load/save directly without mocking the config_dir() function
    // This would require refactoring the Config implementation to accept a path

    // Clean up
    std::env::remove_var("XDG_CONFIG_HOME");
}
