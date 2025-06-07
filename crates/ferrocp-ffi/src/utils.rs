//! FFI utility functions
//!
//! This module provides utility functions for the FFI interface.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::ptr;

/// Convert a Rust Vec<String> to a C array of strings
/// 
/// # Safety
/// 
/// The returned array must be freed with ferrocp_free_string_array.
pub(crate) fn rust_string_vec_to_c_array(strings: Vec<String>) -> (*const *const c_char, usize) {
    if strings.is_empty() {
        return (ptr::null(), 0);
    }

    let mut c_strings: Vec<*const c_char> = Vec::with_capacity(strings.len());
    
    for s in strings {
        if let Ok(c_string) = CString::new(s) {
            c_strings.push(c_string.into_raw() as *const c_char);
        } else {
            c_strings.push(ptr::null());
        }
    }

    let len = c_strings.len();
    let ptr = c_strings.as_ptr();
    std::mem::forget(c_strings); // Prevent deallocation
    
    (ptr, len)
}

/// Free a C array of strings created by rust_string_vec_to_c_array
/// 
/// # Safety
/// 
/// The array must have been created by rust_string_vec_to_c_array.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_free_string_array(
    array: *const *const c_char,
    length: usize,
) {
    if array.is_null() {
        return;
    }

    let slice = std::slice::from_raw_parts(array, length);
    for &ptr in slice {
        if !ptr.is_null() {
            drop(CString::from_raw(ptr as *mut c_char));
        }
    }
    
    // Free the array itself
    drop(Vec::from_raw_parts(array as *mut *const c_char, length, length));
}

/// Get the size of a file in bytes
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_get_file_size(path: *const c_char) -> c_ulonglong {
    if path.is_null() {
        return 0;
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match std::fs::metadata(&path_str) {
        Ok(metadata) => metadata.len(),
        Err(_) => 0,
    }
}

/// Check if a path exists
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_path_exists(path: *const c_char) -> c_int {
    if path.is_null() {
        return 0;
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if std::path::Path::new(&path_str).exists() { 1 } else { 0 }
}

/// Check if a path is a directory
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_is_directory(path: *const c_char) -> c_int {
    if path.is_null() {
        return 0;
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if std::path::Path::new(&path_str).is_dir() { 1 } else { 0 }
}

/// Check if a path is a file
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_is_file(path: *const c_char) -> c_int {
    if path.is_null() {
        return 0;
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if std::path::Path::new(&path_str).is_file() { 1 } else { 0 }
}

/// Get the parent directory of a path
/// 
/// Returns a newly allocated string that must be freed with ferrocp_free_string.
/// Returns null if the path has no parent or is invalid.
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_get_parent_path(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let path_obj = std::path::Path::new(&path_str);
    if let Some(parent) = path_obj.parent() {
        if let Some(parent_str) = parent.to_str() {
            return crate::rust_string_to_c(parent_str.to_string());
        }
    }

    ptr::null_mut()
}

/// Get the filename from a path
/// 
/// Returns a newly allocated string that must be freed with ferrocp_free_string.
/// Returns null if the path has no filename or is invalid.
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_get_filename(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let path_obj = std::path::Path::new(&path_str);
    if let Some(filename) = path_obj.file_name() {
        if let Some(filename_str) = filename.to_str() {
            return crate::rust_string_to_c(filename_str.to_string());
        }
    }

    ptr::null_mut()
}

/// Join two paths
/// 
/// Returns a newly allocated string that must be freed with ferrocp_free_string.
/// Returns null if either path is invalid.
/// 
/// # Safety
/// 
/// Both paths must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_join_paths(
    path1: *const c_char,
    path2: *const c_char,
) -> *mut c_char {
    if path1.is_null() || path2.is_null() {
        return ptr::null_mut();
    }

    let path1_str = match crate::c_string_to_rust(path1) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let path2_str = match crate::c_string_to_rust(path2) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let joined = std::path::Path::new(&path1_str).join(&path2_str);
    if let Some(joined_str) = joined.to_str() {
        crate::rust_string_to_c(joined_str.to_string())
    } else {
        ptr::null_mut()
    }
}

/// Normalize a path (resolve . and .. components)
/// 
/// Returns a newly allocated string that must be freed with ferrocp_free_string.
/// Returns null if the path is invalid.
/// 
/// # Safety
/// 
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ferrocp_normalize_path(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match crate::c_string_to_rust(path) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Simple normalization - in a real implementation, you might want to use
    // a more sophisticated path normalization library
    let path_obj = std::path::Path::new(&path_str);
    if let Ok(canonical) = path_obj.canonicalize() {
        if let Some(canonical_str) = canonical.to_str() {
            return crate::rust_string_to_c(canonical_str.to_string());
        }
    }

    // Fallback to the original path if canonicalization fails
    crate::rust_string_to_c(path_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_string_array_conversion() {
        let strings = vec![
            "hello".to_string(),
            "world".to_string(),
            "test".to_string(),
        ];

        let (array, length) = rust_string_vec_to_c_array(strings);
        assert!(!array.is_null());
        assert_eq!(length, 3);

        unsafe {
            ferrocp_free_string_array(array, length);
        }
    }

    #[test]
    fn test_path_utilities() {
        let test_path = CString::new("/tmp/test.txt").unwrap();
        
        // These tests might fail on Windows, but they demonstrate the API
        unsafe {
            let exists = ferrocp_path_exists(test_path.as_ptr());
            let is_file = ferrocp_is_file(test_path.as_ptr());
            let is_dir = ferrocp_is_directory(test_path.as_ptr());
            
            // Just check that the functions don't crash
            let _ = (exists, is_file, is_dir);
        }
    }

    #[test]
    fn test_path_manipulation() {
        let test_path = CString::new("/tmp/test.txt").unwrap();
        
        unsafe {
            let parent = ferrocp_get_parent_path(test_path.as_ptr());
            if !parent.is_null() {
                let parent_str = CStr::from_ptr(parent).to_str().unwrap();
                assert!(parent_str.contains("tmp") || parent_str.contains("\\"));
                ferrocp_free_string(parent);
            }

            let filename = ferrocp_get_filename(test_path.as_ptr());
            if !filename.is_null() {
                let filename_str = CStr::from_ptr(filename).to_str().unwrap();
                assert_eq!(filename_str, "test.txt");
                ferrocp_free_string(filename);
            }
        }
    }

    #[test]
    fn test_path_joining() {
        let path1 = CString::new("/tmp").unwrap();
        let path2 = CString::new("test.txt").unwrap();
        
        unsafe {
            let joined = ferrocp_join_paths(path1.as_ptr(), path2.as_ptr());
            if !joined.is_null() {
                let joined_str = CStr::from_ptr(joined).to_str().unwrap();
                assert!(joined_str.contains("test.txt"));
                ferrocp_free_string(joined);
            }
        }
    }
}
