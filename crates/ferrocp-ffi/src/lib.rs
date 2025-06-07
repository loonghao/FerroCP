//! C-compatible FFI interface for FerroCP
//!
//! This crate provides a C-compatible Foreign Function Interface (FFI) for FerroCP,
//! enabling integration with Python, C++, and other languages that can call C functions.
//!
//! # Design Principles
//!
//! 1. **C-ABI Compatibility**: All public functions use C calling conventions
//! 2. **Memory Safety**: Proper ownership and lifetime management
//! 3. **Error Handling**: C-style error codes with detailed error information
//! 4. **Thread Safety**: Safe to call from multiple threads
//! 5. **Future Compatibility**: Designed to support Python and C++ bindings
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
//! │   Python API    │    │    C++ API      │    │   C API         │
//! │   (PyO3)        │    │   (cxx/cbindgen)│    │   (Direct)      │
//! └─────────────────┘    └─────────────────┘    └─────────────────┘
//!           │                       │                       │
//!           └───────────────────────┼───────────────────────┘
//!                                   │
//!                         ┌─────────▼─────────┐
//!                         │   FerroCP FFI     │
//!                         │   (This Crate)    │
//!                         └─────────┬─────────┘
//!                                   │
//!                         ┌─────────▼─────────┐
//!                         │  FerroCP Engine   │
//!                         │  (Core Logic)     │
//!                         └───────────────────┘
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::ptr;
use std::sync::Arc;

pub mod types;
pub mod engine;
pub mod error;
pub mod callbacks;
pub mod utils;

pub use types::*;
pub use engine::*;
pub use error::*;
pub use callbacks::*;

/// FFI-safe result type
#[repr(C)]
pub struct FerrocpResult {
    /// Error code (0 = success, non-zero = error)
    pub error_code: c_int,
    /// Error message (null if success)
    pub error_message: *const c_char,
    /// Additional error details as JSON string (null if not available)
    pub error_details: *const c_char,
}

/// FFI-safe copy statistics
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpStats {
    /// Number of files copied
    pub files_copied: c_ulonglong,
    /// Number of directories created
    pub directories_created: c_ulonglong,
    /// Total bytes copied
    pub bytes_copied: c_ulonglong,
    /// Number of files skipped
    pub files_skipped: c_ulonglong,
    /// Number of errors encountered
    pub errors: c_ulonglong,
    /// Duration in milliseconds
    pub duration_ms: c_ulonglong,
    /// Transfer rate in MB/s
    pub transfer_rate_mbps: f64,
    /// Performance efficiency percentage
    pub efficiency_percent: f64,
}

/// FFI-safe device information
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FerrocpDeviceInfo {
    /// Device type as string
    pub device_type: *const c_char,
    /// Filesystem type
    pub filesystem: *const c_char,
    /// Total space in bytes
    pub total_space: c_ulonglong,
    /// Available space in bytes
    pub available_space: c_ulonglong,
    /// Theoretical read speed in MB/s
    pub read_speed_mbps: f64,
    /// Theoretical write speed in MB/s
    pub write_speed_mbps: f64,
}

/// FFI-safe copy request
#[repr(C)]
pub struct FerrocpCopyRequest {
    /// Source path
    pub source: *const c_char,
    /// Destination path
    pub destination: *const c_char,
    /// Copy mode (0=copy, 1=move, 2=sync)
    pub mode: c_int,
    /// Enable compression
    pub compress: c_int,
    /// Preserve metadata
    pub preserve_metadata: c_int,
    /// Verify copy
    pub verify_copy: c_int,
    /// Number of threads (0 = auto)
    pub threads: c_uint,
    /// Buffer size in bytes (0 = auto)
    pub buffer_size: c_ulonglong,
}

/// Progress callback function type
pub type ProgressCallback = extern "C" fn(
    progress_percent: f64,
    bytes_copied: c_ulonglong,
    total_bytes: c_ulonglong,
    current_file: *const c_char,
    user_data: *mut std::ffi::c_void,
);

/// Error callback function type
pub type ErrorCallback = extern "C" fn(
    error_code: c_int,
    error_message: *const c_char,
    file_path: *const c_char,
    user_data: *mut std::ffi::c_void,
);

/// Initialize FerroCP library
/// 
/// This function must be called before using any other FerroCP functions.
/// It initializes the async runtime and internal state.
///
/// # Returns
/// 
/// 0 on success, non-zero error code on failure.
#[no_mangle]
pub extern "C" fn ferrocp_init() -> c_int {
    // Initialize logging and async runtime
    match crate::engine::initialize_runtime() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Failed to initialize FerroCP: {}", e);
            -1
        }
    }
}

/// Cleanup FerroCP library
/// 
/// This function should be called when done using FerroCP to cleanup resources.
#[no_mangle]
pub extern "C" fn ferrocp_cleanup() {
    crate::engine::cleanup_runtime();
}

/// Get library version
/// 
/// Returns a null-terminated string containing the library version.
/// The returned string is statically allocated and should not be freed.
#[no_mangle]
pub extern "C" fn ferrocp_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Free a string allocated by FerroCP
/// 
/// This function should be used to free strings returned by FerroCP functions.
/// 
/// # Safety
/// 
/// The pointer must have been returned by a FerroCP function and not already freed.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Free a FerrocpResult structure
/// 
/// This function frees the memory allocated for error messages in a FerrocpResult.
/// 
/// # Safety
/// 
/// The result must have been returned by a FerroCP function and not already freed.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_free_result(result: *mut FerrocpResult) {
    if !result.is_null() {
        let result = &mut *result;
        if !result.error_message.is_null() {
            ferrocp_free_string(result.error_message as *mut c_char);
            result.error_message = ptr::null();
        }
        if !result.error_details.is_null() {
            ferrocp_free_string(result.error_details as *mut c_char);
            result.error_details = ptr::null();
        }
    }
}

/// Convert Rust string to C string
/// 
/// # Safety
/// 
/// The returned pointer must be freed with ferrocp_free_string.
pub(crate) fn rust_string_to_c(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Convert C string to Rust string
/// 
/// # Safety
/// 
/// The pointer must be a valid null-terminated C string.
pub(crate) unsafe fn c_string_to_rust(ptr: *const c_char) -> Result<String, std::str::Utf8Error> {
    if ptr.is_null() {
        return Ok(String::new());
    }
    
    let c_str = CStr::from_ptr(ptr);
    c_str.to_str().map(|s| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = unsafe { CStr::from_ptr(ferrocp_version()) };
        assert!(!version.to_str().unwrap().is_empty());
    }

    #[test]
    fn test_init_cleanup() {
        assert_eq!(ferrocp_init(), 0);
        ferrocp_cleanup();
    }

    #[test]
    fn test_string_conversion() {
        let rust_str = "Hello, World!".to_string();
        let c_str = rust_string_to_c(rust_str.clone());
        assert!(!c_str.is_null());
        
        let converted_back = unsafe { c_string_to_rust(c_str) }.unwrap();
        assert_eq!(rust_str, converted_back);
        
        unsafe { ferrocp_free_string(c_str) };
    }
}
