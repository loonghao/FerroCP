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

pub mod engine;
pub mod executor;
pub mod monitor;
pub mod scheduler;
pub mod selector;
pub mod task;

pub use engine::{CopyEngine, EngineBuilder};
pub use executor::{ExecutorConfig, TaskExecutor};
pub use monitor::{ProgressMonitor, StatisticsCollector};
pub use scheduler::{SchedulerConfig, TaskScheduler};
pub use selector::{EngineSelector, EngineSelectionConfig, EngineSelection, EngineType, EngineSelectionStats};
pub use task::{CopyRequest, CopyResult, Task, TaskId, TaskStatus};
