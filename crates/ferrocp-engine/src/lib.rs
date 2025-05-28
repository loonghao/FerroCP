//! Main copy engine for FerroCP
//!
//! This crate provides the main copy engine that orchestrates all FerroCP components
//! to deliver high-performance file copying with advanced features.
//!
//! # Features
//!
//! - **Unified API**: Single interface for all copy operations
//! - **Task Management**: Parallel processing with intelligent scheduling
//! - **Progress Tracking**: Real-time progress reporting and statistics
//! - **Error Recovery**: Robust error handling with retry mechanisms
//! - **Configuration**: Flexible configuration system
//! - **Extensibility**: Plugin architecture for custom operations
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_engine::{CopyEngine, CopyRequest};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = CopyEngine::new().await?;
//! let request = CopyRequest::new("source.txt", "destination.txt");
//! let result = engine.execute(request).await?;
//! println!("Copied {} bytes", result.stats.bytes_copied);
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

use ferrocp_config::Config;
use ferrocp_types::{CopyMode, CopyStats, Error, Priority, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod engine;
pub mod executor;
pub mod scheduler;
pub mod task;
pub mod monitor;

pub use engine::{CopyEngine, EngineBuilder};
pub use executor::{TaskExecutor, ExecutorConfig};
pub use scheduler::{TaskScheduler, SchedulerConfig};
pub use task::{CopyRequest, CopyResult, Task, TaskId, TaskStatus};
pub use monitor::{ProgressMonitor, StatisticsCollector};
