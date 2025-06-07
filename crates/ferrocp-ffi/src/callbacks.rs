//! FFI callback support
//!
//! This module provides callback functionality for progress reporting
//! and error handling across the FFI boundary.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::ptr;
use std::sync::{Arc, Mutex};

use crate::types::*;
use crate::{ProgressCallback, ErrorCallback};

/// Callback context for managing callbacks across FFI boundary
#[derive(Debug)]
pub struct CallbackContext {
    /// Progress callback function
    pub progress_callback: Option<ProgressCallback>,
    /// Error callback function
    pub error_callback: Option<ErrorCallback>,
    /// User data pointer
    pub user_data: *mut std::ffi::c_void,
    /// Current file being processed (owned string for safety)
    pub current_file: Option<CString>,
}

unsafe impl Send for CallbackContext {}
unsafe impl Sync for CallbackContext {}

impl CallbackContext {
    /// Create a new callback context
    pub fn new(
        progress_callback: Option<ProgressCallback>,
        error_callback: Option<ErrorCallback>,
        user_data: *mut std::ffi::c_void,
    ) -> Self {
        Self {
            progress_callback,
            error_callback,
            user_data,
            current_file: None,
        }
    }

    /// Call the progress callback if available
    pub fn call_progress(
        &mut self,
        progress_percent: f64,
        bytes_copied: u64,
        total_bytes: u64,
        current_file: Option<&str>,
    ) {
        if let Some(callback) = self.progress_callback {
            // Update current file if provided
            if let Some(file) = current_file {
                self.current_file = CString::new(file).ok();
            }

            let current_file_ptr = self.current_file
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null());

            // Call the callback
            callback(
                progress_percent,
                bytes_copied as c_ulonglong,
                total_bytes as c_ulonglong,
                current_file_ptr,
                self.user_data,
            );
        }
    }

    /// Call the error callback if available
    pub fn call_error(
        &self,
        error_code: FerrocpErrorCode,
        error_message: &str,
        file_path: Option<&str>,
    ) {
        if let Some(callback) = self.error_callback {
            let message_cstring = CString::new(error_message).unwrap_or_default();
            let file_cstring = file_path.and_then(|p| CString::new(p).ok());
            
            let file_ptr = file_cstring
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null());

            callback(
                error_code as c_int,
                message_cstring.as_ptr(),
                file_ptr,
                self.user_data,
            );
        }
    }
}

/// Thread-safe callback manager
pub type CallbackManager = Arc<Mutex<CallbackContext>>;

/// Create a new callback manager
pub fn create_callback_manager(
    progress_callback: Option<ProgressCallback>,
    error_callback: Option<ErrorCallback>,
    user_data: *mut std::ffi::c_void,
) -> CallbackManager {
    Arc::new(Mutex::new(CallbackContext::new(
        progress_callback,
        error_callback,
        user_data,
    )))
}

/// Helper function to safely call progress callback
pub fn safe_call_progress(
    manager: &CallbackManager,
    progress_percent: f64,
    bytes_copied: u64,
    total_bytes: u64,
    current_file: Option<&str>,
) {
    if let Ok(mut context) = manager.lock() {
        context.call_progress(progress_percent, bytes_copied, total_bytes, current_file);
    }
}

/// Helper function to safely call error callback
pub fn safe_call_error(
    manager: &CallbackManager,
    error_code: FerrocpErrorCode,
    error_message: &str,
    file_path: Option<&str>,
) {
    if let Ok(context) = manager.lock() {
        context.call_error(error_code, error_message, file_path);
    }
}

/// Set a progress callback for an operation
/// 
/// This function allows setting a progress callback that will be called
/// during copy operations to report progress.
/// 
/// # Safety
/// 
/// The callback function must be valid for the duration of the operation.
/// The user_data pointer will be passed to the callback as-is.
#[no_mangle]
pub extern "C" fn ferrocp_set_progress_callback(
    callback: Option<ProgressCallback>,
    user_data: *mut std::ffi::c_void,
) -> *mut CallbackContext {
    let context = Box::new(CallbackContext::new(callback, None, user_data));
    Box::into_raw(context)
}

/// Set an error callback for an operation
/// 
/// This function allows setting an error callback that will be called
/// when errors occur during copy operations.
/// 
/// # Safety
/// 
/// The callback function must be valid for the duration of the operation.
/// The user_data pointer will be passed to the callback as-is.
#[no_mangle]
pub extern "C" fn ferrocp_set_error_callback(
    callback: Option<ErrorCallback>,
    user_data: *mut std::ffi::c_void,
) -> *mut CallbackContext {
    let context = Box::new(CallbackContext::new(None, callback, user_data));
    Box::into_raw(context)
}

/// Set both progress and error callbacks
/// 
/// # Safety
/// 
/// The callback functions must be valid for the duration of the operation.
/// The user_data pointer will be passed to both callbacks as-is.
#[no_mangle]
pub extern "C" fn ferrocp_set_callbacks(
    progress_callback: Option<ProgressCallback>,
    error_callback: Option<ErrorCallback>,
    user_data: *mut std::ffi::c_void,
) -> *mut CallbackContext {
    let context = Box::new(CallbackContext::new(
        progress_callback,
        error_callback,
        user_data,
    ));
    Box::into_raw(context)
}

/// Free a callback context
/// 
/// # Safety
/// 
/// The context must have been created by one of the ferrocp_set_*_callback functions.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_free_callback_context(context: *mut CallbackContext) {
    if !context.is_null() {
        drop(Box::from_raw(context));
    }
}

/// Test callback function for progress (used in tests)
#[cfg(test)]
extern "C" fn test_progress_callback(
    progress_percent: f64,
    bytes_copied: c_ulonglong,
    total_bytes: c_ulonglong,
    current_file: *const c_char,
    user_data: *mut std::ffi::c_void,
) {
    // This is just a test callback, it doesn't do anything
    let _ = (progress_percent, bytes_copied, total_bytes, current_file, user_data);
}

/// Test callback function for errors (used in tests)
#[cfg(test)]
extern "C" fn test_error_callback(
    error_code: c_int,
    error_message: *const c_char,
    file_path: *const c_char,
    user_data: *mut std::ffi::c_void,
) {
    // This is just a test callback, it doesn't do anything
    let _ = (error_code, error_message, file_path, user_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_callback_context_creation() {
        let context = CallbackContext::new(
            Some(test_progress_callback),
            Some(test_error_callback),
            ptr::null_mut(),
        );
        
        assert!(context.progress_callback.is_some());
        assert!(context.error_callback.is_some());
        assert!(context.current_file.is_none());
    }

    #[test]
    fn test_callback_manager() {
        let manager = create_callback_manager(
            Some(test_progress_callback),
            Some(test_error_callback),
            ptr::null_mut(),
        );
        
        // Test progress callback
        safe_call_progress(&manager, 50.0, 1024, 2048, Some("test.txt"));
        
        // Test error callback
        safe_call_error(
            &manager,
            FerrocpErrorCode::FileNotFound,
            "Test error",
            Some("test.txt"),
        );
    }

    #[test]
    fn test_ffi_callback_functions() {
        let context = ferrocp_set_callbacks(
            Some(test_progress_callback),
            Some(test_error_callback),
            ptr::null_mut(),
        );
        
        assert!(!context.is_null());
        
        unsafe {
            ferrocp_free_callback_context(context);
        }
    }

    #[test]
    fn test_callback_with_file() {
        let mut context = CallbackContext::new(
            Some(test_progress_callback),
            None,
            ptr::null_mut(),
        );
        
        context.call_progress(25.0, 512, 2048, Some("example.txt"));
        assert!(context.current_file.is_some());
        
        if let Some(ref file) = context.current_file {
            assert_eq!(file.to_str().unwrap(), "example.txt");
        }
    }
}
