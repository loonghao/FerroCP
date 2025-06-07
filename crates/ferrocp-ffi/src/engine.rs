//! FFI engine interface
//!
//! This module provides the main FFI interface for the FerroCP engine,
//! designed to be called from C, Python, C++, and other languages.

use std::os::raw::{c_char, c_int, c_ulonglong};
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;

use ferrocp_engine::{CopyEngine, CopyRequest};
use ferrocp_types::CopyStats;
use tokio::runtime::Runtime;

use crate::types::*;
use crate::{FerrocpResult, FerrocpStats, FerrocpDeviceInfo, FerrocpCopyRequest};
use crate::{ProgressCallback, ErrorCallback};

/// Global runtime for async operations
static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

/// Global engine instances
static ENGINES: OnceLock<Arc<Mutex<HashMap<u64, Arc<CopyEngine>>>>> = OnceLock::new();

/// Engine handle counter
static HANDLE_COUNTER: OnceLock<Arc<Mutex<u64>>> = OnceLock::new();

/// Engine handle type
pub type EngineHandle = u64;

/// Initialize the async runtime
pub(crate) fn initialize_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    RUNTIME.set(Arc::new(runtime)).map_err(|_| "Runtime already initialized")?;
    
    ENGINES.set(Arc::new(Mutex::new(HashMap::new())))
        .map_err(|_| "Engines map already initialized")?;
    
    HANDLE_COUNTER.set(Arc::new(Mutex::new(1)))
        .map_err(|_| "Handle counter already initialized")?;
    
    Ok(())
}

/// Cleanup the runtime
pub(crate) fn cleanup_runtime() {
    // Clear all engines
    if let Some(engines) = ENGINES.get() {
        if let Ok(mut engines) = engines.lock() {
            engines.clear();
        }
    }
}

/// Get the global runtime
fn get_runtime() -> Option<Arc<Runtime>> {
    RUNTIME.get().cloned()
}

/// Generate a new engine handle
fn generate_handle() -> EngineHandle {
    if let Some(counter) = HANDLE_COUNTER.get() {
        if let Ok(mut counter) = counter.lock() {
            let handle = *counter;
            *counter += 1;
            return handle;
        }
    }
    0 // Fallback, should not happen
}

/// Store an engine and return its handle
fn store_engine(engine: Arc<CopyEngine>) -> EngineHandle {
    let handle = generate_handle();
    if let Some(engines) = ENGINES.get() {
        if let Ok(mut engines) = engines.lock() {
            engines.insert(handle, engine);
        }
    }
    handle
}

/// Get an engine by handle
fn get_engine(handle: EngineHandle) -> Option<Arc<CopyEngine>> {
    ENGINES.get()?.lock().ok()?.get(&handle).cloned()
}

/// Remove an engine by handle
fn remove_engine(handle: EngineHandle) -> Option<Arc<CopyEngine>> {
    ENGINES.get()?.lock().ok()?.remove(&handle)
}

/// Create a new FerroCP engine
/// 
/// Returns a handle to the engine, or 0 on failure.
#[no_mangle]
pub extern "C" fn ferrocp_engine_create() -> EngineHandle {
    let runtime = match get_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    match runtime.block_on(async { CopyEngine::new().await }) {
        Ok(engine) => store_engine(Arc::new(engine)),
        Err(_) => 0,
    }
}

/// Destroy a FerroCP engine
/// 
/// Frees all resources associated with the engine handle.
#[no_mangle]
pub extern "C" fn ferrocp_engine_destroy(handle: EngineHandle) {
    remove_engine(handle);
}

/// Execute a copy operation
/// 
/// # Safety
/// 
/// All string pointers in the request must be valid null-terminated C strings.
/// The result must be freed with ferrocp_free_result.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_copy(
    handle: EngineHandle,
    request: *const FerrocpCopyRequest,
    result: *mut FerrocpResult,
) -> c_int {
    if request.is_null() || result.is_null() {
        return FerrocpErrorCode::InvalidArgument as c_int;
    }

    let engine = match get_engine(handle) {
        Some(engine) => engine,
        None => {
            (*result).error_code = FerrocpErrorCode::InvalidArgument as c_int;
            (*result).error_message = crate::rust_string_to_c("Invalid engine handle".to_string());
            return FerrocpErrorCode::InvalidArgument as c_int;
        }
    };

    let runtime = match get_runtime() {
        Some(rt) => rt,
        None => {
            (*result).error_code = FerrocpErrorCode::GenericError as c_int;
            (*result).error_message = crate::rust_string_to_c("Runtime not initialized".to_string());
            return FerrocpErrorCode::GenericError as c_int;
        }
    };

    // Convert FFI request to internal request
    let req = &*request;
    let source = match crate::c_string_to_rust(req.source) {
        Ok(s) => s,
        Err(_) => {
            (*result).error_code = FerrocpErrorCode::InvalidArgument as c_int;
            (*result).error_message = crate::rust_string_to_c("Invalid source path".to_string());
            return FerrocpErrorCode::InvalidArgument as c_int;
        }
    };

    let destination = match crate::c_string_to_rust(req.destination) {
        Ok(s) => s,
        Err(_) => {
            (*result).error_code = FerrocpErrorCode::InvalidArgument as c_int;
            (*result).error_message = crate::rust_string_to_c("Invalid destination path".to_string());
            return FerrocpErrorCode::InvalidArgument as c_int;
        }
    };

    let copy_mode = FerrocpCopyMode::from(req.mode).into();
    
    // Create internal copy request
    let copy_request = CopyRequest::new(source, destination)
        .with_mode(copy_mode)
        .preserve_metadata(c_int_to_bool(req.preserve_metadata))
        .verify_copy(c_int_to_bool(req.verify_copy))
        .enable_compression(c_int_to_bool(req.compress));

    // Execute the copy operation
    match runtime.block_on(async { engine.execute(copy_request).await }) {
        Ok(copy_result) => {
            (*result).error_code = FerrocpErrorCode::Success as c_int;
            (*result).error_message = ptr::null();
            (*result).error_details = ptr::null();
            FerrocpErrorCode::Success as c_int
        }
        Err(e) => {
            (*result).error_code = FerrocpErrorCode::GenericError as c_int;
            (*result).error_message = crate::rust_string_to_c(e.to_string());
            (*result).error_details = ptr::null();
            FerrocpErrorCode::GenericError as c_int
        }
    }
}

/// Execute a copy operation with progress callback
/// 
/// # Safety
/// 
/// All string pointers must be valid null-terminated C strings.
/// Callback functions must be valid for the duration of the operation.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_copy_with_progress(
    handle: EngineHandle,
    request: *const FerrocpCopyRequest,
    progress_callback: Option<ProgressCallback>,
    error_callback: Option<ErrorCallback>,
    user_data: *mut std::ffi::c_void,
    result: *mut FerrocpResult,
) -> c_int {
    // For now, delegate to the basic copy function
    // TODO: Implement progress callbacks
    ferrocp_copy(handle, request, result)
}

/// Get device information for a path
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
/// The result must be freed with ferrocp_free_device_info.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_get_device_info(
    path: *const c_char,
    device_info: *mut FerrocpDeviceInfo,
) -> c_int {
    if path.is_null() || device_info.is_null() {
        return FerrocpErrorCode::InvalidArgument as c_int;
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return FerrocpErrorCode::InvalidArgument as c_int,
    };

    // For now, return a placeholder implementation
    // TODO: Implement proper device detection
    (*device_info).device_type = crate::rust_string_to_c("SSD".to_string());
    (*device_info).filesystem = crate::rust_string_to_c("NTFS".to_string());
    (*device_info).total_space = 1024 * 1024 * 1024 * 1024; // 1TB placeholder
    (*device_info).available_space = 512 * 1024 * 1024 * 1024; // 512GB placeholder
    (*device_info).read_speed_mbps = 500.0;
    (*device_info).write_speed_mbps = 450.0;
    FerrocpErrorCode::Success as c_int
}

/// Free device information
/// 
/// # Safety
/// 
/// The device_info must have been returned by ferrocp_get_device_info.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_free_device_info(device_info: *mut FerrocpDeviceInfo) {
    if !device_info.is_null() {
        let info = &mut *device_info;
        if !info.device_type.is_null() {
            crate::ferrocp_free_string(info.device_type as *mut c_char);
            info.device_type = ptr::null();
        }
        if !info.filesystem.is_null() {
            crate::ferrocp_free_string(info.filesystem as *mut c_char);
            info.filesystem = ptr::null();
        }
    }
}

/// Convert internal stats to FFI stats
pub(crate) fn convert_stats_to_ffi(stats: &CopyStats) -> FerrocpStats {
    FerrocpStats {
        files_copied: stats.files_copied,
        directories_created: stats.directories_created,
        bytes_copied: stats.bytes_copied,
        files_skipped: stats.files_skipped,
        errors: stats.errors,
        duration_ms: stats.duration.as_millis() as c_ulonglong,
        transfer_rate_mbps: stats.transfer_rate(),
        efficiency_percent: stats.zerocopy_efficiency() * 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_engine_lifecycle() {
        assert_eq!(crate::ferrocp_init(), 0);
        
        let handle = ferrocp_engine_create();
        assert_ne!(handle, 0);
        
        ferrocp_engine_destroy(handle);
        crate::ferrocp_cleanup();
    }

    #[test]
    fn test_invalid_handle() {
        assert_eq!(crate::ferrocp_init(), 0);
        
        let invalid_handle = 999999;
        let source = CString::new("test_source").unwrap();
        let dest = CString::new("test_dest").unwrap();
        
        let request = FerrocpCopyRequest {
            source: source.as_ptr(),
            destination: dest.as_ptr(),
            mode: 0,
            compress: 0,
            preserve_metadata: 1,
            verify_copy: 0,
            threads: 0,
            buffer_size: 0,
        };
        
        let mut result = FerrocpResult {
            error_code: 0,
            error_message: ptr::null(),
            error_details: ptr::null(),
        };
        
        let error_code = unsafe { ferrocp_copy(invalid_handle, &request, &mut result) };
        assert_ne!(error_code, 0);
        
        unsafe { crate::ferrocp_free_result(&mut result) };
        crate::ferrocp_cleanup();
    }
}
