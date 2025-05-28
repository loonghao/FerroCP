//! Configuration file watcher for hot-reload functionality

use crate::{Config, ConfigError, ConfigLoader, ConfigResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/// Configuration file watcher for hot-reload functionality
pub struct ConfigWatcher {
    config_path: PathBuf,
    current_config: Arc<Mutex<Config>>,
    last_modified: Arc<Mutex<Option<SystemTime>>>,
    poll_interval: Duration,
    running: Arc<Mutex<bool>>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new<P: AsRef<Path>>(config_path: P) -> ConfigResult<Self> {
        let config_path = config_path.as_ref().to_path_buf();

        if !config_path.exists() {
            return Err(ConfigError::Io {
                path: config_path,
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Configuration file not found",
                ),
            });
        }

        let config = ConfigLoader::load_from_file(&config_path)?;
        let last_modified = Self::get_file_modified_time(&config_path)?;

        Ok(Self {
            config_path,
            current_config: Arc::new(Mutex::new(config)),
            last_modified: Arc::new(Mutex::new(Some(last_modified))),
            poll_interval: Duration::from_secs(1),
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Set the polling interval for checking file changes
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Get the current configuration
    pub fn get_config(&self) -> Config {
        self.current_config.lock().unwrap().clone()
    }

    /// Start watching for configuration changes
    pub fn start_watching(&self) -> ConfigResult<()> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Err(ConfigError::watcher("Watcher is already running"));
        }
        *running = true;
        drop(running);

        let config_path = self.config_path.clone();
        let current_config = Arc::clone(&self.current_config);
        let last_modified = Arc::clone(&self.last_modified);
        let poll_interval = self.poll_interval;
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            while *running.lock().unwrap() {
                if let Err(e) =
                    Self::check_and_reload(&config_path, &current_config, &last_modified)
                {
                    eprintln!("Config watcher error: {}", e);
                }

                thread::sleep(poll_interval);
            }
        });

        Ok(())
    }

    /// Stop watching for configuration changes
    pub fn stop_watching(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }

    /// Check if the watcher is currently running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Manually reload the configuration
    pub fn reload(&self) -> ConfigResult<()> {
        let config = ConfigLoader::load_from_file(&self.config_path)?;
        let modified_time = Self::get_file_modified_time(&self.config_path)?;

        {
            let mut current_config = self.current_config.lock().unwrap();
            *current_config = config;
        }

        {
            let mut last_modified = self.last_modified.lock().unwrap();
            *last_modified = Some(modified_time);
        }

        Ok(())
    }

    /// Check for file changes and reload if necessary
    fn check_and_reload(
        config_path: &Path,
        current_config: &Arc<Mutex<Config>>,
        last_modified: &Arc<Mutex<Option<SystemTime>>>,
    ) -> ConfigResult<()> {
        if !config_path.exists() {
            return Ok(()); // File was deleted, keep current config
        }

        let current_modified = Self::get_file_modified_time(config_path)?;
        let should_reload = {
            let last_modified_guard = last_modified.lock().unwrap();
            match *last_modified_guard {
                Some(last) => current_modified > last,
                None => true,
            }
        };

        if should_reload {
            match ConfigLoader::load_from_file(config_path) {
                Ok(new_config) => {
                    {
                        let mut config_guard = current_config.lock().unwrap();
                        *config_guard = new_config;
                    }

                    {
                        let mut last_modified_guard = last_modified.lock().unwrap();
                        *last_modified_guard = Some(current_modified);
                    }

                    println!("Configuration reloaded from {}", config_path.display());
                }
                Err(e) => {
                    eprintln!("Failed to reload configuration: {}", e);
                    // Keep the current configuration on error
                }
            }
        }

        Ok(())
    }

    /// Get the last modified time of a file
    fn get_file_modified_time(path: &Path) -> ConfigResult<SystemTime> {
        let metadata = std::fs::metadata(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        metadata.modified().map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop_watching();
    }
}

/// Configuration watcher builder for more flexible configuration
pub struct ConfigWatcherBuilder {
    config_path: Option<PathBuf>,
    poll_interval: Duration,
    auto_start: bool,
}

impl ConfigWatcherBuilder {
    /// Create a new configuration watcher builder
    pub fn new() -> Self {
        Self {
            config_path: None,
            poll_interval: Duration::from_secs(1),
            auto_start: false,
        }
    }

    /// Set the configuration file path
    pub fn config_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the polling interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Enable auto-start of the watcher
    pub fn auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    /// Build the configuration watcher
    pub fn build(self) -> ConfigResult<ConfigWatcher> {
        let config_path = self
            .config_path
            .ok_or_else(|| ConfigError::validation("Configuration path is required"))?;

        let watcher = ConfigWatcher::new(config_path)?.poll_interval(self.poll_interval);

        if self.auto_start {
            watcher.start_watching()?;
        }

        Ok(watcher)
    }
}

impl Default for ConfigWatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_watcher_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "performance:\n  enable_zero_copy: true").unwrap();
        temp_file.flush().unwrap();

        let watcher = ConfigWatcher::new(temp_file.path()).unwrap();
        let config = watcher.get_config();
        assert!(config.performance.enable_zero_copy);
    }

    #[test]
    fn test_config_watcher_reload() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "performance:\n  enable_zero_copy: true").unwrap();
        temp_file.flush().unwrap();

        let watcher = ConfigWatcher::new(temp_file.path()).unwrap();

        // Modify the file by truncating and rewriting
        temp_file.seek(std::io::SeekFrom::Start(0)).unwrap();
        temp_file.as_file_mut().set_len(0).unwrap();
        writeln!(temp_file, "performance:\n  enable_zero_copy: false").unwrap();
        temp_file.flush().unwrap();

        // Manually reload
        watcher.reload().unwrap();
        let config = watcher.get_config();
        assert!(!config.performance.enable_zero_copy);
    }

    #[test]
    fn test_config_watcher_builder() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "performance:\n  enable_zero_copy: true").unwrap();
        temp_file.flush().unwrap();

        let watcher = ConfigWatcherBuilder::new()
            .config_path(temp_file.path())
            .poll_interval(Duration::from_millis(100))
            .build()
            .unwrap();

        let config = watcher.get_config();
        assert!(config.performance.enable_zero_copy);
    }
}
