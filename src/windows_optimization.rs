//! Windows-specific optimizations for file operations
//!
//! This module provides Windows-specific optimizations using native Windows APIs
//! for better performance compared to cross-platform implementations.

#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{FILETIME, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_DIRECTORY,
    WIN32_FIND_DATAW,
};

#[cfg(windows)]
use crate::error::{Error, Result};

/// Windows-specific file metadata for fast comparison
#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct FastFileMetadata {
    pub size: u64,
    pub modified_time: SystemTime,
    pub is_directory: bool,
}

#[cfg(windows)]
impl FastFileMetadata {
    /// Create from WIN32_FIND_DATAW
    fn from_find_data(find_data: &WIN32_FIND_DATAW) -> Self {
        let size = ((find_data.nFileSizeHigh as u64) << 32) | (find_data.nFileSizeLow as u64);

        let modified_time = filetime_to_system_time(&find_data.ftLastWriteTime);

        let is_directory = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

        Self {
            size,
            modified_time,
            is_directory,
        }
    }
}

/// Convert Windows FILETIME to SystemTime
#[cfg(windows)]
fn filetime_to_system_time(filetime: &FILETIME) -> SystemTime {
    // Windows FILETIME is 100-nanosecond intervals since January 1, 1601 UTC
    let intervals = ((filetime.dwHighDateTime as u64) << 32) | (filetime.dwLowDateTime as u64);

    // Convert to seconds since Unix epoch (January 1, 1970 UTC)
    // The difference between 1601 and 1970 is 11644473600 seconds
    const FILETIME_UNIX_DIFF: u64 = 11644473600;
    const INTERVALS_PER_SECOND: u64 = 10_000_000;

    if intervals >= FILETIME_UNIX_DIFF * INTERVALS_PER_SECOND {
        let unix_intervals = intervals - (FILETIME_UNIX_DIFF * INTERVALS_PER_SECOND);
        let seconds = unix_intervals / INTERVALS_PER_SECOND;
        let nanos = ((unix_intervals % INTERVALS_PER_SECOND) * 100) as u32;

        UNIX_EPOCH + std::time::Duration::new(seconds, nanos)
    } else {
        UNIX_EPOCH
    }
}

/// Windows-specific fast directory traversal with metadata
#[cfg(windows)]
pub struct FastDirectoryTraversal {
    base_path: PathBuf,
}

#[cfg(windows)]
impl FastDirectoryTraversal {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    /// Traverse directory and collect file metadata using Windows FindFirstFile/FindNextFile
    /// This is significantly faster than using std::fs::read_dir + metadata calls
    pub fn collect_files_with_metadata(&self) -> Result<Vec<(PathBuf, FastFileMetadata)>> {
        let mut results = Vec::new();
        self.traverse_recursive(&self.base_path, &mut results)?;
        Ok(results)
    }

    fn traverse_recursive(
        &self,
        current_path: &Path,
        results: &mut Vec<(PathBuf, FastFileMetadata)>,
    ) -> Result<()> {
        let search_pattern = current_path.join("*");
        let search_pattern_wide = to_wide_string(search_pattern.as_os_str());

        unsafe {
            let mut find_data = WIN32_FIND_DATAW::default();
            let handle = FindFirstFileW(
                PCWSTR(search_pattern_wide.as_ptr()),
                &mut find_data,
            );

            if let Ok(handle) = handle {
                if handle == INVALID_HANDLE_VALUE {
                    return Err(Error::other(format!(
                        "Failed to open directory: {:?}",
                        current_path
                    )));
                }

                loop {
                    let filename = from_wide_string(&find_data.cFileName);

                    // Skip . and .. entries
                    if filename != "." && filename != ".." {
                        let file_path = current_path.join(&filename);
                        let metadata = FastFileMetadata::from_find_data(&find_data);

                        if metadata.is_directory {
                            // Recursively traverse subdirectory
                            self.traverse_recursive(&file_path, results)?;
                        } else {
                            // Add file to results
                            results.push((file_path, metadata));
                        }
                    }

                    // Get next file
                    if FindNextFileW(handle, &mut find_data).is_err() {
                        break;
                    }
                }

                let _ = FindClose(handle);
            } else {
                return Err(Error::other(format!(
                    "Failed to open directory: {:?}",
                    current_path
                )));
            }
        }

        Ok(())
    }
}

/// Convert OsStr to wide string (UTF-16) for Windows APIs
#[cfg(windows)]
fn to_wide_string(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// Convert wide string (UTF-16) from Windows APIs to String
#[cfg(windows)]
fn from_wide_string(wide: &[u16; 260]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

/// Fast skip check using Windows-specific metadata
#[cfg(windows)]
pub fn should_skip_file_fast(
    source_metadata: &FastFileMetadata,
    dest_path: &Path,
) -> Result<bool> {
    // Get destination metadata using Windows API
    let dest_metadata = match get_file_metadata_fast(dest_path)? {
        Some(metadata) => metadata,
        None => return Ok(false), // Destination doesn't exist, don't skip
    };

    // Fast size comparison first
    if dest_metadata.size != source_metadata.size {
        return Ok(false);
    }

    // Then check modification time
    Ok(dest_metadata.modified_time >= source_metadata.modified_time)
}

/// Get file metadata using Windows FindFirstFile (faster than std::fs::metadata)
#[cfg(windows)]
pub fn get_file_metadata_fast(path: &Path) -> Result<Option<FastFileMetadata>> {
    let path_wide = to_wide_string(path.as_os_str());

    unsafe {
        let mut find_data = WIN32_FIND_DATAW::default();
        let handle = FindFirstFileW(
            PCWSTR(path_wide.as_ptr()),
            &mut find_data,
        );

        if let Ok(handle) = handle {
            if handle == INVALID_HANDLE_VALUE {
                return Ok(None); // File doesn't exist
            }

            let metadata = FastFileMetadata::from_find_data(&find_data);
            let _ = FindClose(handle);

            Ok(Some(metadata))
        } else {
            Ok(None) // File doesn't exist
        }
    }
}

/// Batch skip check for multiple files using Windows-specific optimizations
#[cfg(windows)]
pub fn batch_skip_check_fast(
    file_pairs: Vec<(PathBuf, PathBuf, FastFileMetadata)>,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files_to_copy = Vec::new();

    for (source_path, dest_path, source_metadata) in file_pairs {
        match should_skip_file_fast(&source_metadata, &dest_path) {
            Ok(true) => {
                // Skip this file
                tracing::debug!("Skipping file: {:?}", source_path);
            }
            Ok(false) => {
                // Copy this file
                files_to_copy.push((source_path, dest_path));
            }
            Err(_) => {
                // On error, include the file for copying
                files_to_copy.push((source_path, dest_path));
            }
        }
    }

    Ok(files_to_copy)
}

// Non-Windows implementations (fallback to standard methods)
#[cfg(not(windows))]
pub struct FastDirectoryTraversal {
    base_path: std::path::PathBuf,
}

#[cfg(not(windows))]
impl FastDirectoryTraversal {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    pub fn collect_files_with_metadata(&self) -> crate::error::Result<Vec<(std::path::PathBuf, std::fs::Metadata)>> {
        // Fallback to standard walkdir implementation
        use walkdir::WalkDir;
        let mut results = Vec::new();

        for entry in WalkDir::new(&self.base_path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    results.push((entry.path().to_path_buf(), metadata));
                }
            }
        }

        Ok(results)
    }
}
