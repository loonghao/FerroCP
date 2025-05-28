//! Configuration types for FerroCP
//!
//! This module provides type-safe configuration structures with validation
//! and serialization support.

// Serde is imported conditionally through cfg_attr
use std::time::Duration;

/// Buffer size configuration with compile-time validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BufferSize(usize);

impl BufferSize {
    /// Minimum buffer size (4KB)
    pub const MIN: usize = 4 * 1024;
    /// Maximum buffer size (64MB)
    pub const MAX: usize = 64 * 1024 * 1024;
    /// Default buffer size (8MB)
    pub const DEFAULT: usize = 8 * 1024 * 1024;

    /// Create a new buffer size with validation
    pub fn new(size: usize) -> Result<Self, String> {
        if size < Self::MIN {
            Err(format!("Buffer size {} is below minimum {}", size, Self::MIN))
        } else if size > Self::MAX {
            Err(format!("Buffer size {} exceeds maximum {}", size, Self::MAX))
        } else if !size.is_power_of_two() {
            Err(format!("Buffer size {} must be a power of two", size))
        } else {
            Ok(Self(size))
        }
    }

    /// Get the buffer size value
    pub fn get(self) -> usize {
        self.0
    }
}

impl Default for BufferSize {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// Thread count configuration with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThreadCount(usize);

impl ThreadCount {
    /// Minimum thread count
    pub const MIN: usize = 1;
    /// Maximum thread count
    pub const MAX: usize = 256;

    /// Create a new thread count with validation
    pub fn new(count: usize) -> Result<Self, String> {
        if count < Self::MIN {
            Err(format!("Thread count {} is below minimum {}", count, Self::MIN))
        } else if count > Self::MAX {
            Err(format!("Thread count {} exceeds maximum {}", count, Self::MAX))
        } else {
            Ok(Self(count))
        }
    }

    /// Get the thread count value
    pub fn get(self) -> usize {
        self.0
    }

    /// Get the optimal thread count for the current system
    pub fn optimal() -> Self {
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self(cpu_count.min(Self::MAX))
    }
}

impl Default for ThreadCount {
    fn default() -> Self {
        Self::optimal()
    }
}

/// Compression level with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    /// No compression
    pub const NONE: u8 = 0;
    /// Fastest compression
    pub const FASTEST: u8 = 1;
    /// Default compression
    pub const DEFAULT: u8 = 6;
    /// Best compression
    pub const BEST: u8 = 22;

    /// Create a new compression level with validation
    pub fn new(level: u8) -> Result<Self, String> {
        if level > Self::BEST {
            Err(format!("Compression level {} exceeds maximum {}", level, Self::BEST))
        } else {
            Ok(Self(level))
        }
    }

    /// Get the compression level value
    pub fn get(self) -> u8 {
        self.0
    }

    /// Check if compression is enabled
    pub fn is_enabled(self) -> bool {
        self.0 > Self::NONE
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// Retry configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(
        max_retries: u32,
        initial_delay: Duration,
        max_delay: Duration,
        backoff_multiplier: f64,
    ) -> Result<Self, String> {
        if backoff_multiplier <= 1.0 {
            return Err("Backoff multiplier must be greater than 1.0".to_string());
        }
        if initial_delay > max_delay {
            return Err("Initial delay cannot be greater than max delay".to_string());
        }
        Ok(Self {
            max_retries,
            initial_delay,
            max_delay,
            backoff_multiplier,
        })
    }

    /// Calculate the delay for a given retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64);
        Duration::from_millis(delay_ms as u64)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeoutConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Read timeout
    pub read_timeout: Duration,
    /// Write timeout
    pub write_timeout: Duration,
    /// Overall operation timeout
    pub operation_timeout: Option<Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
            operation_timeout: None,
        }
    }
}
