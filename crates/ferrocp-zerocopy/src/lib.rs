//! Next-generation zero-copy operations for FerroCP
//!
//! This crate provides high-performance zero-copy file operations with platform-specific
//! optimizations and hardware acceleration support. It includes:
//!
//! - **Cross-platform zero-copy**: Support for Linux, Windows, and macOS zero-copy APIs
//! - **Hardware acceleration**: Utilize platform-specific hardware features
//! - **Intelligent fallback**: Automatic fallback to optimized copy methods
//! - **Performance monitoring**: Track zero-copy operation effectiveness
//!
//! # Features
//!
//! - `serde` (default): Enable serialization support for zero-copy results
//!
//! # Platform Support
//!
//! - **Linux**: `copy_file_range`, `sendfile`, `io_uring`, BTRFS/XFS reflink
//! - **Windows**: ReFS Copy-on-Write, NTFS hardlinks
//! - **macOS**: `copyfile`, APFS cloning
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_zerocopy::{ZeroCopyEngine, ZeroCopyEngineImpl};
//! use ferrocp_types::{ZeroCopyEngine as ZeroCopyEngineTrait, ZeroCopyMethod};
//!
//! # tokio_test::block_on(async {
//! let engine = ZeroCopyEngineImpl::new();
//! let result = engine.zero_copy("source.txt", "dest.txt", 1024).await?;
//!
//! if result {
//!     println!("Zero-copy operation successful!");
//! } else {
//!     println!("Fallback to regular copy");
//! }
//! # Ok::<(), ferrocp_types::Error>(())
//! # });
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod engine;
pub mod hardware;
pub mod methods;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Re-export main types
pub use engine::{ZeroCopyEngine, ZeroCopyEngineImpl};
pub use hardware::{AccelerationCapabilities, HardwareAcceleration};
pub use methods::{ZeroCopyCapabilities, ZeroCopyMethod};
