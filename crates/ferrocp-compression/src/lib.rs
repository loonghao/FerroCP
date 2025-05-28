//! Adaptive compression system for FerroCP
//!
//! This crate provides a comprehensive compression system with support for multiple algorithms,
//! adaptive compression strategies, and performance optimization. It includes:
//!
//! - **Multi-algorithm support**: zstd, lz4, brotli compression algorithms
//! - **Adaptive compression**: Automatic algorithm and level selection based on data characteristics
//! - **Performance optimization**: Streaming compression, parallel processing, and memory efficiency
//! - **Benchmarking**: Built-in performance testing and comparison tools
//!
//! # Features
//!
//! - `serde` (default): Enable serialization support for compression configurations
//!
//! # Supported Algorithms
//!
//! - **Zstandard (zstd)**: High compression ratio with good speed
//! - **LZ4**: Ultra-fast compression with moderate ratio
//! - **Brotli**: Excellent compression ratio for text data
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_compression::{CompressionEngineImpl, AdaptiveCompressor};
//! use ferrocp_types::{CompressionEngine, CompressionAlgorithm};
//!
//! # tokio_test::block_on(async {
//! let engine = CompressionEngineImpl::new();
//! let data = b"Hello, world! This is test data for compression.";
//!
//! let compressed = engine.compress(data).await?;
//! let decompressed = engine.decompress(&compressed).await?;
//!
//! assert_eq!(data, decompressed.as_slice());
//! # Ok::<(), ferrocp_types::Error>(())
//! # });
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod adaptive;
pub mod algorithms;
pub mod benchmarks;
pub mod engine;
pub mod streaming;

// Re-export main types
pub use adaptive::{AdaptiveCompressor, CompressionStrategy};
pub use algorithms::{Algorithm, AlgorithmImpl};
pub use benchmarks::{BenchmarkResult, CompressionBenchmark};
pub use engine::{CompressionConfig, CompressionEngineImpl, CompressionStats};
pub use streaming::{StreamingCompressor, StreamingDecompressor};
