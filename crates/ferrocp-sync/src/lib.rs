//! Intelligent incremental synchronization for FerroCP
//!
//! This crate provides advanced incremental synchronization capabilities for FerroCP with features like:
//!
//! - **File Difference Detection**: Efficient algorithms to detect file changes and differences
//! - **Incremental Sync**: Only transfer changed parts of files to minimize bandwidth usage
//! - **Hash Caching**: Persistent caching of file hashes to speed up subsequent sync operations
//! - **Delta Compression**: Advanced delta compression algorithms for optimal transfer efficiency
//! - **Conflict Resolution**: Intelligent handling of file conflicts during synchronization
//! - **Progress Tracking**: Real-time progress reporting for sync operations
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_sync::{SyncEngine, SyncRequest, SyncOptions};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut engine = SyncEngine::new().await?;
//! let request = SyncRequest::new("source_dir", "dest_dir")
//!     .with_options(SyncOptions::incremental());
//! let result = engine.sync(request).await?;
//! println!("Synced {} files, {} bytes transferred", result.files_synced, result.bytes_transferred);
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod cache;
pub mod conflict;
pub mod delta;
pub mod diff;
pub mod engine;
pub mod progress;

pub use cache::{CacheEntry, HashCache};
pub use conflict::{ConflictResolution, ConflictResolver, ConflictType};
pub use delta::{DeltaEngine, DeltaPatch};
pub use diff::{ChangeType, DiffEngine, FileChange};
pub use engine::{SyncEngine, SyncOptions, SyncRequest, SyncResult};
pub use progress::{ProgressReporter, SyncProgress};
