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
            Err(format!(
                "Buffer size {} is below minimum {}",
                size,
                Self::MIN
            ))
        } else if size > Self::MAX {
            Err(format!(
                "Buffer size {} exceeds maximum {}",
                size,
                Self::MAX
            ))
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
            Err(format!(
                "Thread count {} is below minimum {}",
                count,
                Self::MIN
            ))
        } else if count > Self::MAX {
            Err(format!(
                "Thread count {} exceeds maximum {}",
                count,
                Self::MAX
            ))
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
            Err(format!(
                "Compression level {} exceeds maximum {}",
                level,
                Self::BEST
            ))
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

        let delay_ms =
            self.initial_delay.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;

    // Property tests for BufferSize
    proptest! {
        #[test]
        fn test_buffer_size_power_of_two_invariant(
            exp in 12u32..26u32  // 2^12 = 4KB to 2^26 = 64MB
        ) {
            let size = 1usize << exp;
            if size >= BufferSize::MIN && size <= BufferSize::MAX {
                let buffer_size = BufferSize::new(size).unwrap();
                prop_assert_eq!(buffer_size.get(), size);
                prop_assert!(buffer_size.get().is_power_of_two());
            }
        }

        #[test]
        fn test_buffer_size_rejects_invalid_sizes(
            size in 1usize..BufferSize::MIN
        ) {
            prop_assert!(BufferSize::new(size).is_err());
        }

        #[test]
        fn test_buffer_size_rejects_non_power_of_two(
            base in 12u32..25u32
        ) {
            let size = (1usize << base) + 1; // Not a power of two
            if size <= BufferSize::MAX {
                prop_assert!(BufferSize::new(size).is_err());
            }
        }
    }

    // Property tests for ThreadCount
    proptest! {
        #[test]
        fn test_thread_count_valid_range(
            count in ThreadCount::MIN..=ThreadCount::MAX
        ) {
            let thread_count = ThreadCount::new(count).unwrap();
            prop_assert_eq!(thread_count.get(), count);
            prop_assert!(thread_count.get() >= ThreadCount::MIN);
            prop_assert!(thread_count.get() <= ThreadCount::MAX);
        }

        #[test]
        fn test_thread_count_rejects_invalid(
            count in (ThreadCount::MAX + 1)..1000usize
        ) {
            prop_assert!(ThreadCount::new(count).is_err());
        }
    }

    // Property tests for CompressionLevel
    proptest! {
        #[test]
        fn test_compression_level_valid_range(
            level in CompressionLevel::NONE..=CompressionLevel::BEST
        ) {
            let compression = CompressionLevel::new(level).unwrap();
            prop_assert_eq!(compression.get(), level);
            prop_assert!(compression.get() <= CompressionLevel::BEST);
        }

        #[test]
        fn test_compression_level_rejects_invalid(
            level in (CompressionLevel::BEST + 1)..=255u8
        ) {
            prop_assert!(CompressionLevel::new(level).is_err());
        }

        #[test]
        fn test_compression_level_enabled_check(
            level in CompressionLevel::NONE..=CompressionLevel::BEST
        ) {
            let compression = CompressionLevel::new(level).unwrap();
            prop_assert_eq!(compression.is_enabled(), level > CompressionLevel::NONE);
        }
    }

    // Property tests for RetryConfig
    proptest! {
        #[test]
        fn test_retry_config_valid_creation(
            max_retries in 0u32..100u32,
            initial_delay_ms in 1u64..10000u64,
            max_delay_ms in 10000u64..300000u64,
            backoff_multiplier in 1.1f64..10.0f64
        ) {
            let initial_delay = Duration::from_millis(initial_delay_ms);
            let max_delay = Duration::from_millis(max_delay_ms);

            if initial_delay <= max_delay {
                let config = RetryConfig::new(
                    max_retries,
                    initial_delay,
                    max_delay,
                    backoff_multiplier
                ).unwrap();

                prop_assert_eq!(config.max_retries, max_retries);
                prop_assert_eq!(config.initial_delay, initial_delay);
                prop_assert_eq!(config.max_delay, max_delay);
                prop_assert_eq!(config.backoff_multiplier, backoff_multiplier);
            }
        }

        #[test]
        fn test_retry_config_delay_calculation(
            attempt in 0u32..10u32
        ) {
            let config = RetryConfig::default();
            let delay = config.delay_for_attempt(attempt);

            // Delay should never exceed max_delay
            prop_assert!(delay <= config.max_delay);

            // First attempt should use initial delay
            if attempt == 0 {
                prop_assert_eq!(delay, config.initial_delay);
            }
        }
    }

    // Unit tests for edge cases
    #[test]
    fn test_buffer_size_constants() {
        assert_eq!(BufferSize::MIN, 4 * 1024);
        assert_eq!(BufferSize::MAX, 64 * 1024 * 1024);
        assert_eq!(BufferSize::DEFAULT, 8 * 1024 * 1024);

        // Default should be valid
        let default_buffer = BufferSize::default();
        assert_eq!(default_buffer.get(), BufferSize::DEFAULT);
    }

    #[test]
    fn test_thread_count_optimal() {
        let optimal = ThreadCount::optimal();
        assert!(optimal.get() >= ThreadCount::MIN);
        assert!(optimal.get() <= ThreadCount::MAX);
    }

    #[test]
    fn test_compression_level_constants() {
        assert_eq!(CompressionLevel::NONE, 0);
        assert_eq!(CompressionLevel::FASTEST, 1);
        assert_eq!(CompressionLevel::DEFAULT, 6);
        assert_eq!(CompressionLevel::BEST, 22);

        // Test ordering
        assert!(CompressionLevel::new(CompressionLevel::NONE).unwrap() <
                CompressionLevel::new(CompressionLevel::FASTEST).unwrap());
        assert!(CompressionLevel::new(CompressionLevel::FASTEST).unwrap() <
                CompressionLevel::new(CompressionLevel::DEFAULT).unwrap());
        assert!(CompressionLevel::new(CompressionLevel::DEFAULT).unwrap() <
                CompressionLevel::new(CompressionLevel::BEST).unwrap());
    }

    #[test]
    fn test_retry_config_invalid_backoff() {
        let result = RetryConfig::new(
            3,
            Duration::from_millis(100),
            Duration::from_secs(30),
            1.0, // Invalid: must be > 1.0
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_config_invalid_delay_order() {
        let result = RetryConfig::new(
            3,
            Duration::from_secs(30), // Initial > max
            Duration::from_millis(100),
            2.0,
        );
        assert!(result.is_err());
    }
}
