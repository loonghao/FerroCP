//! Configuration builder for flexible configuration loading

use crate::{Config, ConfigError, ConfigResult};
use config::{ConfigBuilder as ConfigBuilderInner, Environment, File, FileFormat};
use std::path::{Path, PathBuf};

/// Configuration builder for loading configuration from multiple sources
#[derive(Debug)]
pub struct ConfigBuilder {
    inner: ConfigBuilderInner<config::builder::DefaultState>,
    sources: Vec<ConfigSource>,
    env_prefix: Option<String>,
    env_separator: String,
}

#[derive(Debug, Clone)]
enum ConfigSource {
    File { path: PathBuf, format: FileFormat },
    Defaults,
    Environment { prefix: String },
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            inner: config::Config::builder(),
            sources: Vec::new(),
            env_prefix: None,
            env_separator: "__".to_string(),
        }
    }

    /// Add default configuration values
    pub fn add_defaults(mut self) -> Self {
        self.sources.push(ConfigSource::Defaults);
        self
    }

    /// Add a configuration file source
    pub fn add_source_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let format = Self::detect_format(&path);
        self.sources.push(ConfigSource::File { path, format });
        self
    }

    /// Add a configuration file source with explicit format
    pub fn add_source_file_with_format<P: AsRef<Path>>(
        mut self,
        path: P,
        format: FileFormat,
    ) -> Self {
        let path = path.as_ref().to_path_buf();
        self.sources.push(ConfigSource::File { path, format });
        self
    }

    /// Add environment variable source with prefix
    pub fn add_env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        let prefix = prefix.into();
        self.env_prefix = Some(prefix.clone());
        self.sources.push(ConfigSource::Environment { prefix });
        self
    }

    /// Set environment variable separator (default: "__")
    pub fn env_separator<S: Into<String>>(mut self, separator: S) -> Self {
        self.env_separator = separator.into();
        self
    }

    /// Build the configuration
    pub fn build(mut self) -> ConfigResult<Config> {
        // Start with defaults as the base configuration
        let defaults = Config::default();

        // Convert defaults to a config source
        let defaults_value = serde_yaml::to_value(&defaults)
            .map_err(|e| ConfigError::other(format!("Failed to serialize defaults: {}", e)))?;
        self.inner = self
            .inner
            .add_source(config::Config::try_from(&defaults_value)?);

        // Add file sources
        for source in &self.sources {
            match source {
                ConfigSource::File { path, format } => {
                    if path.exists() {
                        self.inner = self
                            .inner
                            .add_source(File::from(path.clone()).format(*format));
                    }
                }
                ConfigSource::Environment { prefix } => {
                    self.inner = self.inner.add_source(
                        Environment::with_prefix(prefix).separator(&self.env_separator),
                    );
                }
                ConfigSource::Defaults => {
                    // Already handled above
                }
            }
        }

        // Build and deserialize
        let config = self.inner.build()?;
        let result: Config = config.try_deserialize()?;

        // Validate the configuration
        Self::validate(&result)?;

        Ok(result)
    }

    /// Try to build the configuration, returning defaults on error
    pub fn build_or_default(self) -> Config {
        self.build().unwrap_or_default()
    }

    /// Detect file format from extension
    fn detect_format(path: &Path) -> FileFormat {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yaml") | Some("yml") => FileFormat::Yaml,
            Some("toml") => FileFormat::Toml,
            Some("json") => FileFormat::Json,
            _ => FileFormat::Yaml, // Default to YAML
        }
    }

    /// Validate the configuration
    fn validate(config: &Config) -> ConfigResult<()> {
        // Validate buffer size
        if config.performance.buffer_size.get() < 4096 {
            return Err(ConfigError::validation(
                "Buffer size must be at least 4096 bytes",
            ));
        }

        // Validate thread count
        if config.performance.thread_count.get() == 0 {
            return Err(ConfigError::validation(
                "Thread count must be greater than 0",
            ));
        }

        // Validate compression level
        if config.compression.level.get() > 22 {
            return Err(ConfigError::validation(
                "Compression level must be between 0 and 22",
            ));
        }

        // Validate network configuration
        if config.network.max_connections == 0 {
            return Err(ConfigError::validation(
                "Maximum connections must be greater than 0",
            ));
        }

        // Validate logging configuration
        if !["trace", "debug", "info", "warn", "error"].contains(&config.logging.level.as_str()) {
            return Err(ConfigError::validation(
                "Log level must be one of: trace, debug, info, warn, error",
            ));
        }

        // Validate security configuration
        if config.security.max_path_length < 256 {
            return Err(ConfigError::validation(
                "Maximum path length must be at least 256",
            ));
        }

        Ok(())
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_builder_defaults() {
        let config = ConfigBuilder::new().add_defaults().build().unwrap();
        assert!(config.performance.enable_zero_copy);
        assert_eq!(
            config.compression.algorithm,
            ferrocp_types::CompressionAlgorithm::Zstd
        );
    }

    #[test]
    fn test_builder_yaml_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
performance:
  enable_zero_copy: false
  thread_count: 8
compression:
  algorithm: Lz4
"#
        )
        .unwrap();

        let config = ConfigBuilder::new()
            .add_defaults()
            .add_source_file(temp_file.path())
            .build()
            .unwrap();

        assert!(!config.performance.enable_zero_copy);
        assert_eq!(config.performance.thread_count.get(), 8);
        assert_eq!(
            config.compression.algorithm,
            ferrocp_types::CompressionAlgorithm::Lz4
        );
    }

    #[test]
    fn test_builder_validation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
performance:
  thread_count: 0
"#
        )
        .unwrap();

        let result = ConfigBuilder::new()
            .add_defaults()
            .add_source_file(temp_file.path())
            .build();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Thread count must be greater than 0"));
    }
}
