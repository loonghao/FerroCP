//! Adaptive compression strategies
//!
//! This module implements intelligent compression algorithm selection based on
//! data characteristics, performance requirements, and historical performance.

use crate::algorithms::AlgorithmImpl;
use ferrocp_types::CompressionAlgorithm;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Compression strategy for different scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompressionStrategy {
    /// Prioritize compression speed
    Speed,
    /// Balance speed and compression ratio
    Balanced,
    /// Prioritize compression ratio
    Ratio,
    /// Optimize for specific data type
    DataTypeOptimized,
}

/// Data type classification for compression optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DataType {
    /// Plain text data
    Text,
    /// Binary executable or library
    Binary,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
    /// Archive or compressed data
    Archive,
    /// Database files
    Database,
    /// Log files
    Log,
    /// Configuration files
    Config,
    /// Unknown or mixed data
    Unknown,
}

/// Performance metrics for algorithm selection
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PerformanceMetrics {
    /// Average compression ratio
    pub compression_ratio: f64,
    /// Average compression speed (bytes/second)
    pub compression_speed: f64,
    /// Average decompression speed (bytes/second)
    pub decompression_speed: f64,
    /// Number of samples
    pub sample_count: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            compression_ratio: 0.5,
            compression_speed: 1_000_000.0,   // 1MB/s default
            decompression_speed: 5_000_000.0, // 5MB/s default
            sample_count: 0,
        }
    }
}

/// Adaptive compression engine
#[derive(Debug)]
pub struct AdaptiveCompressor {
    /// Current compression strategy
    strategy: CompressionStrategy,
    /// Performance history for each algorithm and data type
    performance_history: HashMap<(CompressionAlgorithm, DataType), PerformanceMetrics>,
    /// File extension to data type mapping
    extension_mapping: HashMap<String, DataType>,
}

impl AdaptiveCompressor {
    /// Create a new adaptive compressor
    pub fn new() -> Self {
        let mut compressor = Self {
            strategy: CompressionStrategy::Balanced,
            performance_history: HashMap::new(),
            extension_mapping: HashMap::new(),
        };

        compressor.initialize_extension_mapping();
        compressor.initialize_default_metrics();
        compressor
    }

    /// Create adaptive compressor with specific strategy
    pub fn with_strategy(strategy: CompressionStrategy) -> Self {
        let mut compressor = Self::new();
        compressor.strategy = strategy;
        compressor
    }

    /// Set compression strategy
    pub fn set_strategy(&mut self, strategy: CompressionStrategy) {
        self.strategy = strategy;
    }

    /// Get current strategy
    pub fn strategy(&self) -> CompressionStrategy {
        self.strategy
    }

    /// Choose the best algorithm and level for given data
    pub fn choose_algorithm(&self, data: &[u8]) -> (CompressionAlgorithm, u8) {
        let data_type = self.classify_data(data);
        debug!("Classified data as {:?}", data_type);

        match self.strategy {
            CompressionStrategy::Speed => self.choose_for_speed(data_type),
            CompressionStrategy::Balanced => self.choose_balanced(data_type),
            CompressionStrategy::Ratio => self.choose_for_ratio(data_type),
            CompressionStrategy::DataTypeOptimized => self.choose_for_data_type(data_type),
        }
    }

    /// Choose algorithm based on file path
    pub fn choose_algorithm_for_path<P: AsRef<Path>>(
        &self,
        path: P,
        data: &[u8],
    ) -> (CompressionAlgorithm, u8) {
        let data_type = self
            .classify_data_by_path(path.as_ref())
            .unwrap_or_else(|| self.classify_data(data));
        debug!("Classified file {:?} as {:?}", path.as_ref(), data_type);

        self.choose_algorithm_for_type(data_type)
    }

    /// Update performance metrics for an algorithm
    pub fn update_performance(
        &mut self,
        algorithm: CompressionAlgorithm,
        data_type: DataType,
        compression_ratio: f64,
        compression_speed: f64,
        decompression_speed: f64,
    ) {
        let key = (algorithm, data_type);
        let metrics = self.performance_history.entry(key).or_default();

        // Update running averages
        let count = metrics.sample_count;
        metrics.compression_ratio =
            (metrics.compression_ratio * count as f64 + compression_ratio) / (count + 1) as f64;
        metrics.compression_speed =
            (metrics.compression_speed * count as f64 + compression_speed) / (count + 1) as f64;
        metrics.decompression_speed =
            (metrics.decompression_speed * count as f64 + decompression_speed) / (count + 1) as f64;
        metrics.sample_count += 1;

        info!(
            "Updated performance for {:?} on {:?}: ratio={:.3}, comp_speed={:.0}, decomp_speed={:.0}",
            algorithm, data_type, compression_ratio, compression_speed, decompression_speed
        );
    }

    /// Classify data type based on content analysis
    fn classify_data(&self, data: &[u8]) -> DataType {
        if data.is_empty() {
            return DataType::Unknown;
        }

        // Sample first 1KB for analysis
        let sample_size = data.len().min(1024);
        let sample = &data[..sample_size];

        // Check for text characteristics
        if self.is_text_data(sample) {
            return DataType::Text;
        }

        // Check for common file signatures
        if let Some(data_type) = self.detect_by_signature(sample) {
            return data_type;
        }

        // Check entropy to distinguish binary from other types
        let entropy = self.calculate_entropy(sample);
        if entropy > 7.5 {
            DataType::Archive // High entropy suggests compressed/encrypted data
        } else if entropy < 4.0 {
            DataType::Text // Low entropy suggests repetitive text
        } else {
            DataType::Binary // Medium entropy suggests binary data
        }
    }

    /// Classify data type based on file path
    fn classify_data_by_path(&self, path: &Path) -> Option<DataType> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| self.extension_mapping.get(&ext.to_lowercase()))
            .copied()
    }

    /// Check if data appears to be text
    fn is_text_data(&self, data: &[u8]) -> bool {
        let text_chars = data
            .iter()
            .filter(|&&b| {
                b.is_ascii_alphanumeric() || b.is_ascii_whitespace() || b.is_ascii_punctuation()
            })
            .count();

        text_chars as f64 / data.len() as f64 > 0.8
    }

    /// Detect data type by file signature
    fn detect_by_signature(&self, data: &[u8]) -> Option<DataType> {
        if data.len() < 4 {
            return None;
        }

        match &data[..4] {
            [0x89, 0x50, 0x4E, 0x47] => Some(DataType::Image), // PNG
            [0xFF, 0xD8, 0xFF, _] => Some(DataType::Image),    // JPEG
            [0x47, 0x49, 0x46, 0x38] => Some(DataType::Image), // GIF
            [0x52, 0x49, 0x46, 0x46] => Some(DataType::Audio), // RIFF (WAV)
            [0x49, 0x44, 0x33, _] => Some(DataType::Audio),    // MP3
            [0x50, 0x4B, 0x03, 0x04] => Some(DataType::Archive), // ZIP
            [0x1F, 0x8B, 0x08, _] => Some(DataType::Archive),  // GZIP
            [0x7F, 0x45, 0x4C, 0x46] => Some(DataType::Binary), // ELF
            [0x4D, 0x5A, _, _] => Some(DataType::Binary),      // PE/DOS
            _ => None,
        }
    }

    /// Calculate Shannon entropy of data
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        let mut counts = [0u32; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Choose algorithm optimized for speed
    fn choose_for_speed(&self, _data_type: DataType) -> (CompressionAlgorithm, u8) {
        // LZ4 is typically fastest
        (CompressionAlgorithm::Lz4, 1)
    }

    /// Choose balanced algorithm
    fn choose_balanced(&self, data_type: DataType) -> (CompressionAlgorithm, u8) {
        match data_type {
            DataType::Text | DataType::Log | DataType::Config => (CompressionAlgorithm::Zstd, 3),
            DataType::Binary | DataType::Database => (CompressionAlgorithm::Lz4, 1),
            DataType::Archive | DataType::Image | DataType::Audio | DataType::Video => {
                (CompressionAlgorithm::None, 0)
            }
            _ => (CompressionAlgorithm::Zstd, 3),
        }
    }

    /// Choose algorithm optimized for compression ratio
    fn choose_for_ratio(&self, data_type: DataType) -> (CompressionAlgorithm, u8) {
        match data_type {
            DataType::Text | DataType::Log | DataType::Config => (CompressionAlgorithm::Brotli, 6),
            DataType::Binary | DataType::Database => (CompressionAlgorithm::Zstd, 6),
            DataType::Archive | DataType::Image | DataType::Audio | DataType::Video => {
                (CompressionAlgorithm::None, 0)
            }
            _ => (CompressionAlgorithm::Zstd, 6),
        }
    }

    /// Choose algorithm optimized for specific data type
    fn choose_for_data_type(&self, data_type: DataType) -> (CompressionAlgorithm, u8) {
        // Use performance history to make informed decisions
        let best = self.find_best_algorithm_for_type(data_type);
        best.unwrap_or_else(|| self.choose_balanced(data_type))
    }

    /// Choose algorithm for specific data type
    fn choose_algorithm_for_type(&self, data_type: DataType) -> (CompressionAlgorithm, u8) {
        match self.strategy {
            CompressionStrategy::Speed => self.choose_for_speed(data_type),
            CompressionStrategy::Balanced => self.choose_balanced(data_type),
            CompressionStrategy::Ratio => self.choose_for_ratio(data_type),
            CompressionStrategy::DataTypeOptimized => self.choose_for_data_type(data_type),
        }
    }

    /// Find best algorithm based on performance history
    fn find_best_algorithm_for_type(
        &self,
        data_type: DataType,
    ) -> Option<(CompressionAlgorithm, u8)> {
        let mut best_score = 0.0;
        let mut best_choice = None;

        for algorithm in AlgorithmImpl::all_algorithms() {
            if algorithm == CompressionAlgorithm::None {
                continue;
            }

            let key = (algorithm, data_type);
            if let Some(metrics) = self.performance_history.get(&key) {
                if metrics.sample_count == 0 {
                    continue;
                }

                // Calculate composite score based on strategy
                let score = match self.strategy {
                    CompressionStrategy::Speed => metrics.compression_speed,
                    CompressionStrategy::Ratio => 1.0 / metrics.compression_ratio,
                    CompressionStrategy::Balanced => {
                        (metrics.compression_speed / 1_000_000.0)
                            * (1.0 / metrics.compression_ratio)
                    }
                    CompressionStrategy::DataTypeOptimized => {
                        (metrics.compression_speed / 1_000_000.0)
                            * (1.0 / metrics.compression_ratio)
                            * metrics.decompression_speed
                            / 1_000_000.0
                    }
                };

                if score > best_score {
                    best_score = score;
                    let algo_impl = AlgorithmImpl::create(algorithm);
                    best_choice = Some((algorithm, algo_impl.default_level()));
                }
            }
        }

        best_choice
    }

    /// Initialize file extension to data type mapping
    fn initialize_extension_mapping(&mut self) {
        let mappings = [
            // Text files
            ("txt", DataType::Text),
            ("md", DataType::Text),
            ("rst", DataType::Text),
            ("csv", DataType::Text),
            ("tsv", DataType::Text),
            ("json", DataType::Text),
            ("xml", DataType::Text),
            ("html", DataType::Text),
            ("htm", DataType::Text),
            ("css", DataType::Text),
            ("js", DataType::Text),
            ("ts", DataType::Text),
            ("py", DataType::Text),
            ("rs", DataType::Text),
            ("c", DataType::Text),
            ("cpp", DataType::Text),
            ("h", DataType::Text),
            ("hpp", DataType::Text),
            // Configuration files
            ("conf", DataType::Config),
            ("cfg", DataType::Config),
            ("ini", DataType::Config),
            ("toml", DataType::Config),
            ("yaml", DataType::Config),
            ("yml", DataType::Config),
            // Log files
            ("log", DataType::Log),
            ("out", DataType::Log),
            ("err", DataType::Log),
            // Images
            ("jpg", DataType::Image),
            ("jpeg", DataType::Image),
            ("png", DataType::Image),
            ("gif", DataType::Image),
            ("bmp", DataType::Image),
            ("tiff", DataType::Image),
            ("webp", DataType::Image),
            ("svg", DataType::Image),
            // Audio
            ("mp3", DataType::Audio),
            ("wav", DataType::Audio),
            ("flac", DataType::Audio),
            ("ogg", DataType::Audio),
            ("aac", DataType::Audio),
            ("m4a", DataType::Audio),
            // Video
            ("mp4", DataType::Video),
            ("avi", DataType::Video),
            ("mkv", DataType::Video),
            ("mov", DataType::Video),
            ("wmv", DataType::Video),
            ("flv", DataType::Video),
            // Archives
            ("zip", DataType::Archive),
            ("rar", DataType::Archive),
            ("7z", DataType::Archive),
            ("tar", DataType::Archive),
            ("gz", DataType::Archive),
            ("bz2", DataType::Archive),
            ("xz", DataType::Archive),
            // Binary
            ("exe", DataType::Binary),
            ("dll", DataType::Binary),
            ("so", DataType::Binary),
            ("dylib", DataType::Binary),
            ("bin", DataType::Binary),
            // Database
            ("db", DataType::Database),
            ("sqlite", DataType::Database),
            ("sqlite3", DataType::Database),
            ("mdb", DataType::Database),
            ("accdb", DataType::Database),
        ];

        for (ext, data_type) in mappings {
            self.extension_mapping.insert(ext.to_string(), data_type);
        }
    }

    /// Initialize default performance metrics
    fn initialize_default_metrics(&mut self) {
        // Initialize with reasonable defaults based on typical algorithm characteristics
        let algorithms = AlgorithmImpl::all_algorithms();
        let data_types = [
            DataType::Text,
            DataType::Binary,
            DataType::Image,
            DataType::Audio,
            DataType::Video,
            DataType::Archive,
            DataType::Database,
            DataType::Log,
            DataType::Config,
            DataType::Unknown,
        ];

        for algorithm in algorithms {
            for data_type in data_types {
                let metrics = match algorithm {
                    CompressionAlgorithm::None => PerformanceMetrics {
                        compression_ratio: 1.0,
                        compression_speed: 100_000_000.0, // Very fast
                        decompression_speed: 100_000_000.0,
                        sample_count: 0,
                    },
                    CompressionAlgorithm::Lz4 => PerformanceMetrics {
                        compression_ratio: 0.7,
                        compression_speed: 50_000_000.0, // Fast
                        decompression_speed: 100_000_000.0,
                        sample_count: 0,
                    },
                    CompressionAlgorithm::Zstd => PerformanceMetrics {
                        compression_ratio: 0.4,
                        compression_speed: 10_000_000.0, // Balanced
                        decompression_speed: 50_000_000.0,
                        sample_count: 0,
                    },
                    CompressionAlgorithm::Brotli => PerformanceMetrics {
                        compression_ratio: 0.3,
                        compression_speed: 2_000_000.0, // Slow but good ratio
                        decompression_speed: 20_000_000.0,
                        sample_count: 0,
                    },
                };

                self.performance_history
                    .insert((algorithm, data_type), metrics);
            }
        }
    }
}

impl Default for AdaptiveCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_compressor_creation() {
        let compressor = AdaptiveCompressor::new();
        assert_eq!(compressor.strategy(), CompressionStrategy::Balanced);
    }

    #[test]
    fn test_strategy_setting() {
        let mut compressor = AdaptiveCompressor::new();
        compressor.set_strategy(CompressionStrategy::Speed);
        assert_eq!(compressor.strategy(), CompressionStrategy::Speed);
    }

    #[test]
    fn test_data_classification() {
        let compressor = AdaptiveCompressor::new();

        // Test text data
        let text_data = b"Hello, world! This is plain text data.";
        assert_eq!(compressor.classify_data(text_data), DataType::Text);

        // Test binary data (random bytes)
        let binary_data = [0x7F, 0x45, 0x4C, 0x46, 0x01, 0x01, 0x01, 0x00]; // ELF header
        assert_eq!(compressor.classify_data(&binary_data), DataType::Binary);
    }

    #[test]
    fn test_path_classification() {
        let compressor = AdaptiveCompressor::new();

        assert_eq!(
            compressor.classify_data_by_path(Path::new("test.txt")),
            Some(DataType::Text)
        );
        assert_eq!(
            compressor.classify_data_by_path(Path::new("image.jpg")),
            Some(DataType::Image)
        );
        assert_eq!(
            compressor.classify_data_by_path(Path::new("archive.zip")),
            Some(DataType::Archive)
        );
        assert_eq!(
            compressor.classify_data_by_path(Path::new("unknown.xyz")),
            None
        );
    }

    #[test]
    fn test_algorithm_selection() {
        let compressor = AdaptiveCompressor::new();
        let text_data = b"This is test text data for compression.";

        let (algorithm, level) = compressor.choose_algorithm(text_data);
        assert_ne!(algorithm, CompressionAlgorithm::None);
        assert!(level > 0);
    }

    #[test]
    fn test_performance_update() {
        let mut compressor = AdaptiveCompressor::new();

        compressor.update_performance(
            CompressionAlgorithm::Zstd,
            DataType::Text,
            0.3,
            5_000_000.0,
            25_000_000.0,
        );

        let key = (CompressionAlgorithm::Zstd, DataType::Text);
        let metrics = compressor.performance_history.get(&key).unwrap();
        assert_eq!(metrics.sample_count, 1);
    }
}
