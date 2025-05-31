//! Python API bindings for FerroCP
//!
//! This crate provides Python bindings for the FerroCP high-performance file copy library.
//! It exposes a Python API that allows users to leverage FerroCP's advanced features from Python code.
//!
//! # Features
//!
//! - **High-Performance Copying**: Leverage FerroCP's optimized file copying algorithms
//! - **Async Support**: Full async/await support for non-blocking operations
//! - **Progress Tracking**: Real-time progress reporting with callbacks
//! - **Configuration**: Flexible configuration options for different use cases
//! - **Cross-Platform**: Works on Windows, macOS, and Linux
//!
//! # Examples
//!
//! ```python
//! import ferrocp
//! import asyncio
//!
//! async def copy_files():
//!     # Simple file copy
//!     await ferrocp.copy_file("source.txt", "destination.txt")
//!
//!     # Copy with progress tracking
//!     def progress_callback(progress):
//!         print(f"Progress: {progress.percentage:.1f}%")
//!
//!     await ferrocp.copy_file(
//!         "large_file.bin",
//!         "backup.bin",
//!         progress_callback=progress_callback
//!     )
//!
//! asyncio.run(copy_files())
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

use pyo3::prelude::*;

pub mod async_support;
pub mod batch_operations;
pub mod config;
pub mod copy;
pub mod error;
pub mod gil_optimization;
pub mod network;
pub mod object_cache;
pub mod progress;
pub mod sync;

use async_support::*;
use batch_operations::*;
use config::*;
use copy::*;
use error::*;
use network::*;
use object_cache::*;
use progress::*;
use sync::*;

/// Python module for FerroCP
#[pymodule]
fn _ferrocp(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Initialize async runtime with proper configuration
    let builder = tokio::runtime::Builder::new_multi_thread();
    pyo3_asyncio::tokio::init(builder);

    // Add classes
    m.add_class::<PyCopyEngine>()?;
    m.add_class::<PyCopyOptions>()?;
    m.add_class::<PyCopyResult>()?;
    m.add_class::<PyProgress>()?;
    m.add_class::<PySyncEngine>()?;
    m.add_class::<PySyncOptions>()?;
    m.add_class::<PyNetworkClient>()?;
    m.add_class::<PyNetworkConfig>()?;
    m.add_class::<PyAsyncOperation>()?;
    m.add_class::<PyAsyncManager>()?;

    // Add batch operation classes
    m.add_class::<PyBatchCopyRequest>()?;
    m.add_class::<PyBatchCopyResult>()?;
    m.add_class::<PyBatchCopyEngine>()?;

    // Add cache classes
    m.add_class::<PyCacheStats>()?;
    m.add_class::<PyCacheConfig>()?;

    // Add exceptions
    m.add("FerrocpError", py.get_type::<PyFerrocpError>())?;
    m.add("IoError", py.get_type::<PyIoError>())?;
    m.add("ConfigError", py.get_type::<PyConfigError>())?;
    m.add("NetworkError", py.get_type::<PyNetworkError>())?;
    m.add("SyncError", py.get_type::<PySyncError>())?;

    // Add convenience functions
    m.add_function(wrap_pyfunction!(copy_file, m)?)?;
    m.add_function(wrap_pyfunction!(copy_directory, m)?)?;
    m.add_function(wrap_pyfunction!(quick_copy, m)?)?;
    m.add_function(wrap_pyfunction!(copy_with_verification, m)?)?;
    m.add_function(wrap_pyfunction!(copy_with_compression, m)?)?;
    m.add_function(wrap_pyfunction!(copy_file_async, m)?)?;
    m.add_function(wrap_pyfunction!(create_async_manager, m)?)?;
    m.add_function(wrap_pyfunction!(sync_directories, m)?)?;
    m.add_function(wrap_pyfunction!(get_version, m)?)?;

    // Add batch operation functions
    m.add_function(wrap_pyfunction!(copy_files_batch, m)?)?;
    m.add_function(wrap_pyfunction!(create_batch_requests, m)?)?;

    // Add cache functions
    m.add_function(wrap_pyfunction!(get_cache_stats, m)?)?;
    m.add_function(wrap_pyfunction!(clear_cache, m)?)?;
    m.add_function(wrap_pyfunction!(configure_cache, m)?)?;

    Ok(())
}
