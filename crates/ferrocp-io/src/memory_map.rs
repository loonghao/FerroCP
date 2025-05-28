//! Memory-mapped file operations for high-performance I/O

use ferrocp_types::{Error, Result};
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::path::Path;
use tracing::debug;

/// Options for memory mapping
#[derive(Debug, Clone)]
pub struct MemoryMapOptions {
    /// Enable read-only mapping
    pub read_only: bool,
    /// Offset in the file to start mapping
    pub offset: u64,
    /// Length of the mapping (None = entire file)
    pub length: Option<usize>,
    /// Enable huge pages if available
    pub huge_pages: bool,
    /// Populate the mapping immediately
    pub populate: bool,
}

impl Default for MemoryMapOptions {
    fn default() -> Self {
        Self {
            read_only: true,
            offset: 0,
            length: None,
            huge_pages: false,
            populate: false,
        }
    }
}

/// Memory-mapped file for efficient large file operations
#[derive(Debug)]
pub struct MemoryMappedFile {
    mmap: Mmap,
    file_size: u64,
    options: MemoryMapOptions,
}

impl MemoryMappedFile {
    /// Create a memory-mapped file with default options
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_options(path, MemoryMapOptions::default())
    }

    /// Create a memory-mapped file with custom options
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        options: MemoryMapOptions,
    ) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error::Io {
            message: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        let metadata = file.metadata().map_err(|e| Error::Io {
            message: format!("Failed to read file metadata: {}", e),
        })?;

        let file_size = metadata.len();

        // Check if file is suitable for memory mapping
        if file_size == 0 {
            return Err(Error::other("Cannot memory map empty file"));
        }

        // Create memory mapping
        let mut mmap_options = MmapOptions::new();
        
        if options.offset > 0 {
            mmap_options.offset(options.offset);
        }

        if let Some(length) = options.length {
            mmap_options.len(length);
        }

        if options.populate {
            mmap_options.populate();
        }

        if options.huge_pages {
            // Note: huge pages support depends on memmap2 version and platform
            // For now, we'll skip this feature to avoid compilation issues
            debug!("Huge pages requested but not implemented in this version");
        }

        // SAFETY: Memory mapping is safe here because:
        // 1. We have a valid file handle
        // 2. The file exists and has been opened successfully
        // 3. We're using the memmap2 crate which provides safe abstractions
        let mmap = unsafe {
            mmap_options.map(&file).map_err(|e| Error::Io {
                message: format!("Failed to create memory mapping: {}", e),
            })?
        };

        debug!(
            "Created memory mapping for '{}': {} bytes at offset {}",
            path.display(),
            mmap.len(),
            options.offset
        );

        Ok(Self {
            mmap,
            file_size,
            options,
        })
    }

    /// Get the mapped data as a byte slice
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }

    /// Get the length of the mapped region
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Check if the mapping is empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Get the original file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get the mapping options
    pub fn options(&self) -> &MemoryMapOptions {
        &self.options
    }

    /// Advise the kernel about memory usage patterns (Unix only)
    #[cfg(unix)]
    pub fn advise(&self, advice: MemoryAdvice) -> Result<()> {
        use memmap2::Advice;

        let mmap_advice = match advice {
            MemoryAdvice::Normal => Advice::Normal,
            MemoryAdvice::Random => Advice::Random,
            MemoryAdvice::Sequential => Advice::Sequential,
            MemoryAdvice::WillNeed => Advice::WillNeed,
            MemoryAdvice::DontNeed => Advice::DontNeed,
        };

        self.mmap.advise(mmap_advice).map_err(|e| Error::Io {
            message: format!("Failed to advise memory usage: {}", e),
        })?;

        debug!("Applied memory advice: {:?}", advice);
        Ok(())
    }

    /// Advise the kernel about memory usage patterns (Windows - no-op)
    #[cfg(windows)]
    pub fn advise(&self, _advice: MemoryAdvice) -> Result<()> {
        debug!("Memory advice not supported on Windows");
        Ok(())
    }

    /// Lock the mapped memory to prevent swapping (Unix only)
    #[cfg(unix)]
    pub fn lock(&self) -> Result<()> {
        self.mmap.lock().map_err(|e| Error::Io {
            message: format!("Failed to lock memory: {}", e),
        })?;

        debug!("Locked memory mapping");
        Ok(())
    }

    /// Lock the mapped memory to prevent swapping (Windows - no-op)
    #[cfg(windows)]
    pub fn lock(&self) -> Result<()> {
        debug!("Memory locking not supported on Windows");
        Ok(())
    }

    /// Unlock the mapped memory (Unix only)
    #[cfg(unix)]
    pub fn unlock(&self) -> Result<()> {
        self.mmap.unlock().map_err(|e| Error::Io {
            message: format!("Failed to unlock memory: {}", e),
        })?;

        debug!("Unlocked memory mapping");
        Ok(())
    }

    /// Unlock the mapped memory (Windows - no-op)
    #[cfg(windows)]
    pub fn unlock(&self) -> Result<()> {
        debug!("Memory unlocking not supported on Windows");
        Ok(())
    }

    /// Check if memory mapping is recommended for a file
    pub fn is_recommended_for_file_size(file_size: u64) -> bool {
        // Memory mapping is generally beneficial for files larger than 64KB
        // but smaller than available memory
        const MIN_SIZE: u64 = 64 * 1024; // 64KB
        const MAX_SIZE: u64 = 1024 * 1024 * 1024; // 1GB (conservative limit)

        file_size >= MIN_SIZE && file_size <= MAX_SIZE
    }

    /// Get optimal chunk size for processing the mapped file
    pub fn optimal_chunk_size(&self) -> usize {
        // Base chunk size on file size and system page size
        let page_size = 4096; // 4KB page size (common on most systems)
        
        if self.len() < 1024 * 1024 {
            // Small files: process in 64KB chunks
            64 * 1024
        } else if self.len() < 100 * 1024 * 1024 {
            // Medium files: process in 1MB chunks
            1024 * 1024
        } else {
            // Large files: process in 4MB chunks
            4 * 1024 * 1024
        }.max(page_size)
    }
}

/// Memory usage advice for the kernel
#[derive(Debug, Clone, Copy)]
pub enum MemoryAdvice {
    /// Normal access pattern
    Normal,
    /// Random access pattern
    Random,
    /// Sequential access pattern
    Sequential,
    /// Will need this memory soon
    WillNeed,
    /// Don't need this memory
    DontNeed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_mapped_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test file for memory mapping.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap_file = MemoryMappedFile::open(temp_file.path()).unwrap();
        
        assert_eq!(mmap_file.len(), test_data.len());
        assert_eq!(mmap_file.file_size(), test_data.len() as u64);
        assert_eq!(mmap_file.as_slice(), test_data);
        assert!(!mmap_file.is_empty());
    }

    #[test]
    fn test_memory_mapped_file_with_options() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test file for memory mapping with options.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let options = MemoryMapOptions {
            read_only: true,
            offset: 7, // Skip "Hello, "
            length: Some(5), // Read "World"
            huge_pages: false,
            populate: false,
        };

        let mmap_file = MemoryMappedFile::open_with_options(temp_file.path(), options).unwrap();
        
        assert_eq!(mmap_file.len(), 5);
        assert_eq!(mmap_file.as_slice(), b"World");
    }

    #[test]
    fn test_memory_advice() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = vec![0u8; 1024 * 1024]; // 1MB of zeros
        temp_file.write_all(&test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap_file = MemoryMappedFile::open(temp_file.path()).unwrap();
        
        // Test different memory advice patterns
        mmap_file.advise(MemoryAdvice::Sequential).unwrap();
        mmap_file.advise(MemoryAdvice::Random).unwrap();
        mmap_file.advise(MemoryAdvice::Normal).unwrap();
    }

    #[test]
    fn test_file_size_recommendation() {
        assert!(!MemoryMappedFile::is_recommended_for_file_size(1024)); // Too small
        assert!(MemoryMappedFile::is_recommended_for_file_size(1024 * 1024)); // Good size
        assert!(!MemoryMappedFile::is_recommended_for_file_size(2 * 1024 * 1024 * 1024)); // Too large
    }

    #[test]
    fn test_optimal_chunk_size() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
        temp_file.write_all(&test_data).unwrap();
        temp_file.flush().unwrap();

        let mmap_file = MemoryMappedFile::open(temp_file.path()).unwrap();
        let chunk_size = mmap_file.optimal_chunk_size();
        
        assert!(chunk_size >= 4096); // At least one page
        assert!(chunk_size <= 4 * 1024 * 1024); // At most 4MB
    }
}
