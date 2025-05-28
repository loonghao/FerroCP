//! High-performance async I/O engine for FerroCP
//!
//! This crate provides a high-performance, async-first I/O engine for FerroCP with features like:
//!
//! - **Smart buffering**: Adaptive buffer sizes based on file size and device type
//! - **Memory mapping**: Efficient handling of large files using memory-mapped I/O
//! - **Zero-copy operations**: Platform-specific optimizations for maximum performance
//! - **Async streaming**: Non-blocking I/O operations with backpressure handling
//! - **Progress tracking**: Real-time progress reporting for long-running operations
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_io::{AsyncFileReader, AsyncFileWriter, BufferedCopyEngine, CopyEngine};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut engine = BufferedCopyEngine::new();
//! let stats = engine.copy_file("source.txt", "destination.txt").await?;
//! println!("Copied {} bytes", stats.bytes_copied);
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod buffer;
pub mod copy;
pub mod memory_map;
pub mod reader;
pub mod stream;
pub mod writer;

pub use buffer::{AdaptiveBuffer, BufferPool, SmartBuffer};
pub use copy::{BufferedCopyEngine, CopyEngine, CopyOptions};
pub use memory_map::{MemoryMapOptions, MemoryMappedFile};
pub use reader::{AsyncFileReader, FileReader};
pub use stream::{FileStream, ProgressStream};
pub use writer::{AsyncFileWriter, FileWriter};
