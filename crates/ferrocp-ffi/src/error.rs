//! FFI error handling
//!
//! This module provides error handling utilities for the FFI interface.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::types::FerrocpErrorCode;
use crate::FerrocpResult;

/// Create a success result
pub(crate) fn create_success_result() -> FerrocpResult {
    FerrocpResult {
        error_code: FerrocpErrorCode::Success as c_int,
        error_message: ptr::null(),
        error_details: ptr::null(),
    }
}

/// Create an error result
pub(crate) fn create_error_result(
    error_code: FerrocpErrorCode,
    message: String,
    details: Option<String>,
) -> FerrocpResult {
    let error_message = crate::rust_string_to_c(message);
    let error_details = details.map(crate::rust_string_to_c).unwrap_or(ptr::null_mut());
    
    FerrocpResult {
        error_code: error_code as c_int,
        error_message,
        error_details,
    }
}

/// Convert a Rust error to an FFI error code
pub(crate) fn rust_error_to_ffi_code(error: &dyn std::error::Error) -> FerrocpErrorCode {
    let error_str = error.to_string().to_lowercase();
    
    if error_str.contains("not found") || error_str.contains("no such file") {
        FerrocpErrorCode::FileNotFound
    } else if error_str.contains("permission") || error_str.contains("access denied") {
        FerrocpErrorCode::PermissionDenied
    } else if error_str.contains("space") || error_str.contains("disk full") {
        FerrocpErrorCode::InsufficientSpace
    } else if error_str.contains("invalid") || error_str.contains("malformed") {
        FerrocpErrorCode::InvalidPath
    } else if error_str.contains("network") || error_str.contains("connection") {
        FerrocpErrorCode::NetworkError
    } else if error_str.contains("compression") || error_str.contains("decompress") {
        FerrocpErrorCode::CompressionError
    } else if error_str.contains("verification") || error_str.contains("checksum") {
        FerrocpErrorCode::VerificationError
    } else if error_str.contains("cancel") || error_str.contains("abort") {
        FerrocpErrorCode::Cancelled
    } else if error_str.contains("timeout") {
        FerrocpErrorCode::Timeout
    } else if error_str.contains("memory") || error_str.contains("allocation") {
        FerrocpErrorCode::OutOfMemory
    } else {
        FerrocpErrorCode::GenericError
    }
}

/// Get error code description
#[no_mangle]
pub extern "C" fn ferrocp_error_code_description(error_code: c_int) -> *const c_char {
    let description = match error_code {
        0 => "Success",
        1 => "Generic error",
        2 => "File not found",
        3 => "Permission denied",
        4 => "Insufficient space",
        5 => "Invalid path",
        6 => "Network error",
        7 => "Compression error",
        8 => "Verification error",
        9 => "Cancelled by user",
        10 => "Invalid argument",
        11 => "Out of memory",
        12 => "Timeout",
        _ => "Unknown error",
    };
    
    // Return static string pointer
    description.as_ptr() as *const c_char
}

/// Check if an error code represents success
#[no_mangle]
pub extern "C" fn ferrocp_is_success(error_code: c_int) -> c_int {
    if error_code == FerrocpErrorCode::Success as c_int { 1 } else { 0 }
}

/// Check if an error code represents a recoverable error
#[no_mangle]
pub extern "C" fn ferrocp_is_recoverable_error(error_code: c_int) -> c_int {
    match error_code {
        // These errors might be recoverable with retry or different approach
        x if x == FerrocpErrorCode::NetworkError as c_int => 1,
        x if x == FerrocpErrorCode::Timeout as c_int => 1,
        x if x == FerrocpErrorCode::InsufficientSpace as c_int => 1,
        // These errors are generally not recoverable
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_error_descriptions() {
        let desc = unsafe { CStr::from_ptr(ferrocp_error_code_description(0)) };
        assert_eq!(desc.to_str().unwrap(), "Success");
        
        let desc = unsafe { CStr::from_ptr(ferrocp_error_code_description(2)) };
        assert_eq!(desc.to_str().unwrap(), "File not found");
        
        let desc = unsafe { CStr::from_ptr(ferrocp_error_code_description(999)) };
        assert_eq!(desc.to_str().unwrap(), "Unknown error");
    }

    #[test]
    fn test_success_check() {
        assert_eq!(ferrocp_is_success(0), 1);
        assert_eq!(ferrocp_is_success(1), 0);
        assert_eq!(ferrocp_is_success(-1), 0);
    }

    #[test]
    fn test_recoverable_check() {
        assert_eq!(ferrocp_is_recoverable_error(FerrocpErrorCode::NetworkError as c_int), 1);
        assert_eq!(ferrocp_is_recoverable_error(FerrocpErrorCode::Timeout as c_int), 1);
        assert_eq!(ferrocp_is_recoverable_error(FerrocpErrorCode::FileNotFound as c_int), 0);
        assert_eq!(ferrocp_is_recoverable_error(FerrocpErrorCode::PermissionDenied as c_int), 0);
    }

    #[test]
    fn test_result_creation() {
        let success = create_success_result();
        assert_eq!(success.error_code, 0);
        assert!(success.error_message.is_null());
        
        let error = create_error_result(
            FerrocpErrorCode::FileNotFound,
            "Test error".to_string(),
            Some("Additional details".to_string()),
        );
        assert_eq!(error.error_code, FerrocpErrorCode::FileNotFound as c_int);
        assert!(!error.error_message.is_null());
        assert!(!error.error_details.is_null());
        
        // Clean up
        unsafe {
            crate::ferrocp_free_string(error.error_message as *mut c_char);
            crate::ferrocp_free_string(error.error_details as *mut c_char);
        }
    }
}
