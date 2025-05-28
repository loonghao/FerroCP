//! Main compression engine implementation
//!
//! This module provides the primary compression engine that implements the
//! CompressionEngine trait and coordinates different compression algorithms.

use crate::adaptive::AdaptiveCompressor;
use crate::algorithms::{Algorithm, AlgorithmImpl};
use ferrocp_types::{CompressionAlgorithm, CompressionEngine, CompressionLevel, Error, Result};

use std::time::{Duration, Instant};
use tracing::{debug, info};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Compression engine configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompressionConfig {
    /// Default compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Default compression level
    pub level: CompressionLevel,
    /// Enable adaptive compression
    pub adaptive: bool,
    /// Minimum file size for compression
    pub min_file_size: u64,
    /// Maximum file size for compression
    pub max_file_size: u64,
    /// Compression timeout
    pub timeout: Duration,
    /// Enable parallel compression for large data
    pub parallel: bool,
    /// Chunk size for parallel compression
    pub chunk_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: CompressionLevel::default(),
            adaptive: true,
            min_file_size: 1024,                     // 1KB minimum
            max_file_size: 100 * 1024 * 1024 * 1024, // 100GB maximum
            timeout: Duration::from_secs(300),       // 5 minutes
            parallel: true,
            chunk_size: 1024 * 1024, // 1MB chunks
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CompressionStats {
    /// Total compression operations
    pub compressions: u64,
    /// Total decompression operations
    pub decompressions: u64,
    /// Total bytes compressed
    pub bytes_compressed: u64,
    /// Total bytes decompressed
    pub bytes_decompressed: u64,
    /// Total compressed size
    pub compressed_size: u64,
    /// Total decompressed size
    pub decompressed_size: u64,
    /// Average compression time
    pub avg_compression_time: Duration,
    /// Average decompression time
    pub avg_decompression_time: Duration,
    /// Average compression ratio
    pub avg_compression_ratio: f64,
}

impl CompressionStats {
    /// Calculate overall compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_compressed == 0 {
            1.0
        } else {
            self.compressed_size as f64 / self.bytes_compressed as f64
        }
    }

    /// Calculate compression efficiency (inverse of ratio)
    pub fn compression_efficiency(&self) -> f64 {
        let ratio = self.compression_ratio();
        if ratio == 0.0 {
            0.0
        } else {
            1.0 / ratio
        }
    }

    /// Update compression statistics
    pub fn update_compression(
        &mut self,
        original_size: u64,
        compressed_size: u64,
        duration: Duration,
    ) {
        self.compressions += 1;
        self.bytes_compressed += original_size;
        self.compressed_size += compressed_size;

        // Update average compression time
        let total_time = self.avg_compression_time.as_nanos() as u64 * (self.compressions - 1)
            + duration.as_nanos() as u64;
        self.avg_compression_time = Duration::from_nanos(total_time / self.compressions);

        // Update average compression ratio
        let ratio = compressed_size as f64 / original_size as f64;
        self.avg_compression_ratio = (self.avg_compression_ratio * (self.compressions - 1) as f64
            + ratio)
            / self.compressions as f64;
    }

    /// Update decompression statistics
    pub fn update_decompression(
        &mut self,
        compressed_size: u64,
        decompressed_size: u64,
        duration: Duration,
    ) {
        self.decompressions += 1;
        self.bytes_decompressed += compressed_size;
        self.decompressed_size += decompressed_size;

        // Update average decompression time
        let total_time = self.avg_decompression_time.as_nanos() as u64 * (self.decompressions - 1)
            + duration.as_nanos() as u64;
        self.avg_decompression_time = Duration::from_nanos(total_time / self.decompressions);
    }
}

/// Main compression engine implementation
pub struct CompressionEngineImpl {
    /// Engine configuration
    config: CompressionConfig,
    /// Engine statistics
    stats: CompressionStats,
    /// Adaptive compressor
    adaptive: AdaptiveCompressor,
    /// Algorithm implementations cache
    algorithms: std::collections::HashMap<CompressionAlgorithm, Box<dyn Algorithm + Send + Sync>>,
}

impl CompressionEngineImpl {
    /// Create a new compression engine
    pub fn new() -> Self {
        Self::with_config(CompressionConfig::default())
    }

    /// Convert algorithm to ID for storage
    fn algorithm_to_id(algorithm: CompressionAlgorithm) -> u8 {
        match algorithm {
            CompressionAlgorithm::None => 0,
            CompressionAlgorithm::Zstd => 1,
            CompressionAlgorithm::Lz4 => 2,
            CompressionAlgorithm::Brotli => 3,
        }
    }

    /// Convert ID back to algorithm
    fn id_to_algorithm(id: u8) -> Result<CompressionAlgorithm> {
        match id {
            0 => Ok(CompressionAlgorithm::None),
            1 => Ok(CompressionAlgorithm::Zstd),
            2 => Ok(CompressionAlgorithm::Lz4),
            3 => Ok(CompressionAlgorithm::Brotli),
            _ => Err(Error::compression(format!(
                "Unknown compression algorithm ID: {}",
                id
            ))),
        }
    }

    /// Create a compression engine with custom configuration
    pub fn with_config(config: CompressionConfig) -> Self {
        let adaptive = AdaptiveCompressor::new();
        let mut algorithms = std::collections::HashMap::new();

        // Pre-create algorithm implementations
        for algorithm_type in AlgorithmImpl::all_algorithms() {
            algorithms.insert(algorithm_type, AlgorithmImpl::create(algorithm_type));
        }

        Self {
            config,
            stats: CompressionStats::default(),
            adaptive,
            algorithms,
        }
    }

    /// Get engine configuration
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }

    /// Get engine statistics
    pub fn stats(&self) -> &CompressionStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = CompressionStats::default();
    }

    /// Update configuration
    pub fn update_config(&mut self, config: CompressionConfig) {
        self.config = config;
    }

    /// Choose the best algorithm for the given data
    fn choose_algorithm(&self, data: &[u8]) -> (CompressionAlgorithm, u8) {
        if self.config.adaptive {
            self.adaptive.choose_algorithm(data)
        } else {
            (self.config.algorithm, self.config.level.get())
        }
    }

    /// Check if compression should be applied
    fn should_compress(&self, data: &[u8]) -> bool {
        let size = data.len() as u64;
        size >= self.config.min_file_size && size <= self.config.max_file_size
    }

    /// Compress data with timeout
    async fn compress_with_timeout(
        &self,
        data: &[u8],
        algorithm: CompressionAlgorithm,
        level: u8,
    ) -> Result<Vec<u8>> {
        // Check if algorithm is available
        if !self.algorithms.contains_key(&algorithm) {
            return Err(Error::compression(format!(
                "Algorithm {:?} not available",
                algorithm
            )));
        }

        let data_clone = data.to_vec();

        // Create a new algorithm instance for the blocking task
        let algo_impl = AlgorithmImpl::create(algorithm);

        let result = tokio::time::timeout(
            self.config.timeout,
            tokio::task::spawn_blocking(move || algo_impl.compress(&data_clone, level)),
        )
        .await;

        match result {
            Ok(Ok(compressed)) => compressed,
            Ok(Err(join_err)) => Err(Error::other(format!("Task join error: {}", join_err))),
            Err(_) => Err(Error::compression("Compression timeout")),
        }
    }

    /// Decompress data with timeout
    async fn decompress_with_timeout(
        &self,
        data: &[u8],
        algorithm: CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        let data_clone = data.to_vec();

        // Create a new algorithm instance for the blocking task
        let algo_impl = AlgorithmImpl::create(algorithm);

        let result = tokio::time::timeout(
            self.config.timeout,
            tokio::task::spawn_blocking(move || algo_impl.decompress(&data_clone)),
        )
        .await;

        match result {
            Ok(Ok(decompressed)) => decompressed,
            Ok(Err(join_err)) => Err(Error::other(format!("Task join error: {}", join_err))),
            Err(_) => Err(Error::compression("Decompression timeout")),
        }
    }
}

impl Default for CompressionEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CompressionEngine for CompressionEngineImpl {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let start_time = Instant::now();

        if !self.should_compress(data) {
            debug!(
                "Skipping compression for {} bytes (below threshold)",
                data.len()
            );
            // For uncompressed data, prepend a special marker (255) to indicate no compression
            let mut result = Vec::with_capacity(data.len() + 1);
            result.push(255); // Special marker for uncompressed data
            result.extend_from_slice(data);
            return Ok(result);
        }

        let (algorithm, level) = self.choose_algorithm(data);
        debug!(
            "Compressing {} bytes with {:?} level {}",
            data.len(),
            algorithm,
            level
        );

        let compressed = self.compress_with_timeout(data, algorithm, level).await?;
        let duration = start_time.elapsed();

        // Prepend algorithm identifier to compressed data
        let mut result = Vec::with_capacity(compressed.len() + 1);
        result.push(Self::algorithm_to_id(algorithm));
        result.extend_from_slice(&compressed);

        // Update statistics (note: this requires mutable access)
        // In a real implementation, you might use interior mutability (Mutex/RwLock)
        info!(
            "Compressed {} bytes to {} bytes ({:.2}% ratio) in {:?}",
            data.len(),
            result.len(),
            (result.len() as f64 / data.len() as f64) * 100.0,
            duration
        );

        Ok(result)
    }

    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let start_time = Instant::now();

        if data.is_empty() {
            return Err(Error::compression("Empty data cannot be decompressed"));
        }

        // Extract algorithm identifier from first byte
        let algorithm_id = data[0];

        // Check for special uncompressed marker
        if algorithm_id == 255 {
            debug!("Data was not compressed, returning original data");
            return Ok(data[1..].to_vec());
        }

        let algorithm = Self::id_to_algorithm(algorithm_id)?;
        let compressed_data = &data[1..];

        match self
            .decompress_with_timeout(compressed_data, algorithm)
            .await
        {
            Ok(decompressed) => {
                let duration = start_time.elapsed();
                debug!(
                    "Decompressed {} bytes to {} bytes with {:?} in {:?}",
                    data.len(),
                    decompressed.len(),
                    algorithm,
                    duration
                );
                Ok(decompressed)
            }
            Err(e) => Err(e),
        }
    }

    fn estimate_compression_ratio(&self, data: &[u8]) -> f64 {
        if !self.should_compress(data) {
            return 1.0;
        }

        let (algorithm, _) = self.choose_algorithm(data);

        if let Some(algo) = self.algorithms.get(&algorithm) {
            algo.estimate_ratio(data)
        } else {
            0.5 // Default estimate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compression_engine_creation() {
        let engine = CompressionEngineImpl::new();
        assert_eq!(engine.config().algorithm, CompressionAlgorithm::Zstd);
        assert!(engine.config().adaptive);
    }

    #[tokio::test]
    async fn test_compression_engine_with_config() {
        let mut config = CompressionConfig::default();
        config.algorithm = CompressionAlgorithm::Lz4;
        config.adaptive = false;

        let engine = CompressionEngineImpl::with_config(config);
        assert_eq!(engine.config().algorithm, CompressionAlgorithm::Lz4);
        assert!(!engine.config().adaptive);
    }

    #[tokio::test]
    async fn test_compression_and_decompression() {
        let engine = CompressionEngineImpl::new();
        let data = b"Hello, world! This is test data for compression.".repeat(10);

        let compressed = engine.compress(&data).await.unwrap();
        let decompressed = engine.decompress(&compressed).await.unwrap();

        assert_eq!(data, decompressed);
    }

    #[tokio::test]
    async fn test_small_data_skip() {
        let engine = CompressionEngineImpl::new();
        let small_data = b"Hi"; // Below minimum threshold

        let result = engine.compress(small_data).await.unwrap();

        // Small data should have uncompressed marker (255) + original data
        assert_eq!(result.len(), small_data.len() + 1);
        assert_eq!(result[0], 255); // Uncompressed marker
        assert_eq!(&result[1..], small_data);

        // Test round-trip
        let decompressed = engine.decompress(&result).await.unwrap();
        assert_eq!(decompressed, small_data);
    }

    #[test]
    fn test_compression_stats() {
        let mut stats = CompressionStats::default();

        stats.update_compression(1000, 500, Duration::from_millis(10));
        assert_eq!(stats.compressions, 1);
        assert_eq!(stats.bytes_compressed, 1000);
        assert_eq!(stats.compressed_size, 500);
        assert_eq!(stats.compression_ratio(), 0.5);

        stats.update_decompression(500, 1000, Duration::from_millis(5));
        assert_eq!(stats.decompressions, 1);
        assert_eq!(stats.bytes_decompressed, 500);
        assert_eq!(stats.decompressed_size, 1000);
    }
}
