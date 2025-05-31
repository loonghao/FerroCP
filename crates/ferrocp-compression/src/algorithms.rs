//! Compression algorithm implementations
//!
//! This module provides implementations for different compression algorithms
//! with a unified interface for easy switching and comparison.

use ferrocp_types::{CompressionAlgorithm, Error, Result};
use std::io::{Read, Write};

/// Trait for compression algorithm implementations
pub trait Algorithm {
    /// Compress data with the specified level
    fn compress(&self, data: &[u8], level: u8) -> Result<Vec<u8>>;

    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Get the algorithm type
    fn algorithm_type(&self) -> CompressionAlgorithm;

    /// Get the default compression level
    fn default_level(&self) -> u8;

    /// Get the maximum compression level
    fn max_level(&self) -> u8;

    /// Estimate compression ratio for given data
    fn estimate_ratio(&self, data: &[u8]) -> f64;

    /// Check if the algorithm is suitable for the given data
    fn is_suitable_for(&self, data: &[u8]) -> bool;
}

/// Algorithm implementation factory
pub struct AlgorithmImpl;

impl AlgorithmImpl {
    /// Create an algorithm implementation for the specified type
    pub fn create(algorithm: CompressionAlgorithm) -> Box<dyn Algorithm + Send + Sync> {
        match algorithm {
            CompressionAlgorithm::None => Box::new(NoCompression),
            CompressionAlgorithm::Zstd => Box::new(ZstdAlgorithm),
            CompressionAlgorithm::Lz4 => Box::new(Lz4Algorithm),
            CompressionAlgorithm::Brotli => Box::new(BrotliAlgorithm),
        }
    }

    /// Get all available algorithms
    pub fn all_algorithms() -> Vec<CompressionAlgorithm> {
        vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
        ]
    }
}

/// No compression implementation
#[derive(Debug, Clone)]
pub struct NoCompression;

impl Algorithm for NoCompression {
    fn compress(&self, data: &[u8], _level: u8) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn algorithm_type(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::None
    }

    fn default_level(&self) -> u8 {
        0
    }

    fn max_level(&self) -> u8 {
        0
    }

    fn estimate_ratio(&self, _data: &[u8]) -> f64 {
        1.0
    }

    fn is_suitable_for(&self, _data: &[u8]) -> bool {
        true
    }
}

/// Zstandard compression implementation
#[derive(Debug, Clone)]
pub struct ZstdAlgorithm;

impl Algorithm for ZstdAlgorithm {
    fn compress(&self, data: &[u8], level: u8) -> Result<Vec<u8>> {
        let level = level.min(self.max_level()) as i32;
        zstd::bulk::compress(data, level)
            .map_err(|e| Error::compression(format!("Zstd compression failed: {}", e)))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::bulk::decompress(data, 100 * 1024 * 1024) // 100MB limit
            .map_err(|e| Error::compression(format!("Zstd decompression failed: {}", e)))
    }

    fn algorithm_type(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Zstd
    }

    fn default_level(&self) -> u8 {
        3
    }

    fn max_level(&self) -> u8 {
        22
    }

    fn estimate_ratio(&self, data: &[u8]) -> f64 {
        // Simple heuristic based on data characteristics
        let sample_size = data.len().min(1024);
        let sample = &data[..sample_size];

        // Count unique bytes
        let mut unique_bytes = [false; 256];
        for &byte in sample {
            unique_bytes[byte as usize] = true;
        }
        let unique_count = unique_bytes.iter().filter(|&&x| x).count();

        // Estimate based on entropy
        let entropy_ratio = unique_count as f64 / 256.0;
        0.3 + (entropy_ratio * 0.5) // Zstd typically achieves 30-80% of original size
    }

    fn is_suitable_for(&self, data: &[u8]) -> bool {
        // Zstd is good for most data types, especially text and structured data
        data.len() >= 64 // Minimum size for effective compression
    }
}

/// LZ4 compression implementation
#[derive(Debug, Clone)]
pub struct Lz4Algorithm;

impl Algorithm for Lz4Algorithm {
    fn compress(&self, data: &[u8], _level: u8) -> Result<Vec<u8>> {
        Ok(lz4_flex::compress_prepend_size(data))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| Error::compression(format!("LZ4 decompression failed: {}", e)))
    }

    fn algorithm_type(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Lz4
    }

    fn default_level(&self) -> u8 {
        1
    }

    fn max_level(&self) -> u8 {
        1 // LZ4 doesn't have compression levels
    }

    fn estimate_ratio(&self, data: &[u8]) -> f64 {
        // LZ4 focuses on speed over compression ratio
        let sample_size = data.len().min(1024);
        let sample = &data[..sample_size];

        // Simple repetition detection
        let mut repetitions = 0;
        for i in 1..sample.len() {
            if sample[i] == sample[i - 1] {
                repetitions += 1;
            }
        }

        let repetition_ratio = repetitions as f64 / sample.len() as f64;
        0.6 + (repetition_ratio * 0.3) // LZ4 typically achieves 60-90% of original size
    }

    fn is_suitable_for(&self, data: &[u8]) -> bool {
        // LZ4 is good when speed is more important than compression ratio
        data.len() >= 32
    }
}

/// Brotli compression implementation
#[derive(Debug, Clone)]
pub struct BrotliAlgorithm;

impl Algorithm for BrotliAlgorithm {
    fn compress(&self, data: &[u8], level: u8) -> Result<Vec<u8>> {
        let level = level.min(self.max_level()) as u32;
        let mut compressed = Vec::new();
        let mut compressor = brotli::CompressorWriter::new(&mut compressed, 4096, level, 22);

        compressor
            .write_all(data)
            .map_err(|e| Error::compression(format!("Brotli compression write failed: {}", e)))?;

        compressor
            .flush()
            .map_err(|e| Error::compression(format!("Brotli compression flush failed: {}", e)))?;

        drop(compressor);
        Ok(compressed)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        let mut decompressor = brotli::Decompressor::new(data, 4096);

        decompressor
            .read_to_end(&mut decompressed)
            .map_err(|e| Error::compression(format!("Brotli decompression failed: {}", e)))?;

        Ok(decompressed)
    }

    fn algorithm_type(&self) -> CompressionAlgorithm {
        CompressionAlgorithm::Brotli
    }

    fn default_level(&self) -> u8 {
        6
    }

    fn max_level(&self) -> u8 {
        11
    }

    fn estimate_ratio(&self, data: &[u8]) -> f64 {
        // Brotli excels with text data
        let sample_size = data.len().min(1024);
        let sample = &data[..sample_size];

        // Check for text-like characteristics
        let text_chars = sample
            .iter()
            .filter(|&&b| b.is_ascii_alphanumeric() || b.is_ascii_whitespace())
            .count();

        let text_ratio = text_chars as f64 / sample.len() as f64;

        if text_ratio > 0.8 {
            0.2 + (text_ratio * 0.3) // Excellent compression for text
        } else {
            0.4 + (text_ratio * 0.4) // Good compression for mixed data
        }
    }

    fn is_suitable_for(&self, data: &[u8]) -> bool {
        // Brotli is excellent for text data and web content
        if data.len() < 128 {
            return false;
        }

        // Check if data looks like text
        let sample_size = data.len().min(512);
        let sample = &data[..sample_size];
        let text_chars = sample
            .iter()
            .filter(|&&b| {
                b.is_ascii_alphanumeric() || b.is_ascii_whitespace() || b.is_ascii_punctuation()
            })
            .count();

        text_chars as f64 / sample.len() as f64 > 0.7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_compression() {
        let algo = NoCompression;
        let data = b"Hello, world!";

        let compressed = algo.compress(data, 0).unwrap();
        let decompressed = algo.decompress(&compressed).unwrap();

        assert_eq!(data, compressed.as_slice());
        assert_eq!(data, decompressed.as_slice());
        assert_eq!(algo.algorithm_type(), CompressionAlgorithm::None);
    }

    #[test]
    fn test_zstd_compression() {
        let algo = ZstdAlgorithm;
        let data = b"Hello, world! This is a test string for compression.".repeat(10);

        let compressed = algo.compress(&data, algo.default_level()).unwrap();
        let decompressed = algo.decompress(&compressed).unwrap();

        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len());
        assert_eq!(algo.algorithm_type(), CompressionAlgorithm::Zstd);
    }

    #[test]
    fn test_lz4_compression() {
        let algo = Lz4Algorithm;
        let data = b"Hello, world! This is a test string for compression.".repeat(5);

        let compressed = algo.compress(&data, algo.default_level()).unwrap();
        let decompressed = algo.decompress(&compressed).unwrap();

        assert_eq!(data, decompressed);
        assert_eq!(algo.algorithm_type(), CompressionAlgorithm::Lz4);
    }

    #[test]
    fn test_brotli_compression() {
        let algo = BrotliAlgorithm;
        let data =
            b"Hello, world! This is a test string for compression with lots of text content."
                .repeat(3);

        let compressed = algo.compress(&data, algo.default_level()).unwrap();
        let decompressed = algo.decompress(&compressed).unwrap();

        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len());
        assert_eq!(algo.algorithm_type(), CompressionAlgorithm::Brotli);
    }

    #[test]
    fn test_algorithm_factory() {
        let algorithms = AlgorithmImpl::all_algorithms();
        assert_eq!(algorithms.len(), 4);

        for algorithm_type in algorithms {
            let algo = AlgorithmImpl::create(algorithm_type);
            assert_eq!(algo.algorithm_type(), algorithm_type);
        }
    }
}
