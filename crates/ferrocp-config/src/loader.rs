//! Configuration loader utilities

use crate::{Config, ConfigBuilder, ConfigError, ConfigResult};
use std::path::{Path, PathBuf};

/// Configuration loader with common loading patterns
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from default locations
    pub fn load_default() -> ConfigResult<Config> {
        let mut builder = ConfigBuilder::new().add_defaults();

        // Try common configuration file locations
        let config_paths = Self::get_default_config_paths();

        for path in config_paths {
            if path.exists() {
                builder = builder.add_source_file(&path);
                break; // Use the first found config file
            }
        }

        // Add environment variables
        builder = builder.add_env_prefix("FERROCP");

        builder.build()
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Config> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Configuration file not found",
                ),
            });
        }

        ConfigBuilder::new()
            .add_defaults()
            .add_source_file(path)
            .add_env_prefix("FERROCP")
            .build()
    }

    /// Load configuration from multiple files (later files override earlier ones)
    pub fn load_from_files<P: AsRef<Path>>(paths: &[P]) -> ConfigResult<Config> {
        let mut builder = ConfigBuilder::new().add_defaults();

        for path in paths {
            let path = path.as_ref();
            if path.exists() {
                builder = builder.add_source_file(path);
            }
        }

        builder = builder.add_env_prefix("FERROCP");
        builder.build()
    }

    /// Load configuration with custom environment prefix
    pub fn load_with_env_prefix<S: Into<String>>(prefix: S) -> ConfigResult<Config> {
        let mut builder = ConfigBuilder::new().add_defaults();

        // Try default config file locations
        let config_paths = Self::get_default_config_paths();
        for path in config_paths {
            if path.exists() {
                builder = builder.add_source_file(&path);
                break;
            }
        }

        builder = builder.add_env_prefix(prefix);
        builder.build()
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(config: &Config, path: P) -> ConfigResult<()> {
        let path = path.as_ref();

        // Determine format from file extension
        let content = match path.extension().and_then(|ext| ext.to_str()) {
            Some("yaml") | Some("yml") => {
                serde_yaml::to_string(config).map_err(|e| ConfigError::Serialization {
                    message: format!("Failed to serialize to YAML: {}", e),
                })?
            }
            Some("toml") => {
                toml::to_string_pretty(config).map_err(|e| ConfigError::Serialization {
                    message: format!("Failed to serialize to TOML: {}", e),
                })?
            }
            Some("json") => {
                serde_json::to_string_pretty(config).map_err(|e| ConfigError::Serialization {
                    message: format!("Failed to serialize to JSON: {}", e),
                })?
            }
            _ => {
                // Default to YAML
                serde_yaml::to_string(config).map_err(|e| ConfigError::Serialization {
                    message: format!("Failed to serialize to YAML: {}", e),
                })?
            }
        };

        std::fs::write(path, content).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    /// Generate a default configuration file
    pub fn generate_default_config<P: AsRef<Path>>(path: P) -> ConfigResult<()> {
        let config = Config::default();
        Self::save_to_file(&config, path)
    }

    /// Get default configuration file paths in order of preference
    fn get_default_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Current directory
        paths.push(PathBuf::from("ferrocp.yaml"));
        paths.push(PathBuf::from("ferrocp.yml"));
        paths.push(PathBuf::from("ferrocp.toml"));
        paths.push(PathBuf::from(".ferrocp.yaml"));
        paths.push(PathBuf::from(".ferrocp.yml"));
        paths.push(PathBuf::from(".ferrocp.toml"));

        // User config directory
        if let Some(config_dir) = dirs::config_dir() {
            let ferrocp_dir = config_dir.join("ferrocp");
            paths.push(ferrocp_dir.join("config.yaml"));
            paths.push(ferrocp_dir.join("config.yml"));
            paths.push(ferrocp_dir.join("config.toml"));
        }

        // System config directory (Unix-like systems)
        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/etc/ferrocp/config.yaml"));
            paths.push(PathBuf::from("/etc/ferrocp/config.yml"));
            paths.push(PathBuf::from("/etc/ferrocp/config.toml"));
        }

        paths
    }

    /// Check if a configuration file exists in default locations
    pub fn config_exists() -> Option<PathBuf> {
        Self::get_default_config_paths()
            .into_iter()
            .find(|path| path.exists())
    }

    /// Validate a configuration file without loading it
    pub fn validate_file<P: AsRef<Path>>(path: P) -> ConfigResult<()> {
        let _config = Self::load_from_file(path)?;
        Ok(())
    }
}

// Add dirs dependency for cross-platform config directory detection
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME").ok().map(|home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
            })
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|home| PathBuf::from(home).join(".config"))
                })
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_default() {
        let config = ConfigLoader::load_default().unwrap();
        assert!(config.performance.enable_zero_copy);
    }

    #[test]
    fn test_save_and_load_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");

        let original_config = Config::default();
        ConfigLoader::save_to_file(&original_config, &config_path).unwrap();

        let loaded_config = ConfigLoader::load_from_file(&config_path).unwrap();
        assert_eq!(
            original_config.performance.enable_zero_copy,
            loaded_config.performance.enable_zero_copy
        );
    }

    #[test]
    fn test_save_and_load_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");

        let original_config = Config::default();
        ConfigLoader::save_to_file(&original_config, &config_path).unwrap();

        let loaded_config = ConfigLoader::load_from_file(&config_path).unwrap();
        assert_eq!(
            original_config.performance.enable_zero_copy,
            loaded_config.performance.enable_zero_copy
        );
    }

    #[test]
    fn test_generate_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("default.yaml");

        ConfigLoader::generate_default_config(&config_path).unwrap();
        assert!(config_path.exists());

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        assert!(config.performance.enable_zero_copy);
    }
}
