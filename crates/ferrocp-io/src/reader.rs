//! Async file reader with smart buffering

use crate::AdaptiveBuffer;
use ferrocp_types::{Error, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tracing::debug;

/// Async file reader with adaptive buffering
#[derive(Debug)]
pub struct AsyncFileReader {
    reader: BufReader<File>,
    file_size: u64,
    bytes_read: u64,
}

impl AsyncFileReader {
    /// Open a file for reading
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).await.map_err(|e| Error::Io {
            message: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        let metadata = file.metadata().await.map_err(|e| Error::Io {
            message: format!("Failed to read file metadata: {}", e),
        })?;

        let file_size = metadata.len();
        let reader = BufReader::new(file);

        debug!("Opened file for reading: {} ({} bytes)", path.display(), file_size);

        Ok(Self {
            reader,
            file_size,
            bytes_read: 0,
        })
    }

    /// Read data into an adaptive buffer
    pub async fn read_into_buffer(&mut self, buffer: &mut AdaptiveBuffer) -> Result<usize> {
        // Ensure buffer has capacity
        if buffer.capacity() == 0 {
            buffer.reserve(64 * 1024); // Default 64KB
        }

        // Read data into buffer
        let bytes_read = self.reader.read_buf(buffer.as_mut()).await.map_err(|e| Error::Io {
            message: format!("Failed to read from file: {}", e),
        })?;

        self.bytes_read += bytes_read as u64;
        
        debug!("Read {} bytes from file ({} total)", bytes_read, self.bytes_read);
        Ok(bytes_read)
    }

    /// Read a specific amount of data
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.reader.read_exact(buf).await.map_err(|e| Error::Io {
            message: format!("Failed to read exact amount: {}", e),
        })?;

        self.bytes_read += buf.len() as u64;
        Ok(())
    }

    /// Read all remaining data
    pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let bytes_read = self.reader.read_to_end(buf).await.map_err(|e| Error::Io {
            message: format!("Failed to read to end: {}", e),
        })?;

        self.bytes_read += bytes_read as u64;
        Ok(bytes_read)
    }

    /// Get the total file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get the number of bytes read so far
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Get the reading progress as a percentage
    pub fn progress(&self) -> f64 {
        if self.file_size > 0 {
            (self.bytes_read as f64 / self.file_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if we've reached the end of the file
    pub fn is_eof(&self) -> bool {
        self.bytes_read >= self.file_size
    }
}

/// Synchronous file reader for compatibility
#[derive(Debug)]
pub struct FileReader {
    file: std::fs::File,
    file_size: u64,
    bytes_read: u64,
}

impl FileReader {
    /// Open a file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|e| Error::Io {
            message: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        let metadata = file.metadata().map_err(|e| Error::Io {
            message: format!("Failed to read file metadata: {}", e),
        })?;

        let file_size = metadata.len();

        debug!("Opened file for reading: {} ({} bytes)", path.display(), file_size);

        Ok(Self {
            file,
            file_size,
            bytes_read: 0,
        })
    }

    /// Read data into a buffer
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        
        let bytes_read = self.file.read(buf).map_err(|e| Error::Io {
            message: format!("Failed to read from file: {}", e),
        })?;

        self.bytes_read += bytes_read as u64;
        Ok(bytes_read)
    }

    /// Read exact amount of data
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        use std::io::Read;
        
        self.file.read_exact(buf).map_err(|e| Error::Io {
            message: format!("Failed to read exact amount: {}", e),
        })?;

        self.bytes_read += buf.len() as u64;
        Ok(())
    }

    /// Read all remaining data
    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        use std::io::Read;
        
        let bytes_read = self.file.read_to_end(buf).map_err(|e| Error::Io {
            message: format!("Failed to read to end: {}", e),
        })?;

        self.bytes_read += bytes_read as u64;
        Ok(bytes_read)
    }

    /// Get the total file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get the number of bytes read so far
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Get the reading progress as a percentage
    pub fn progress(&self) -> f64 {
        if self.file_size > 0 {
            (self.bytes_read as f64 / self.file_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if we've reached the end of the file
    pub fn is_eof(&self) -> bool {
        self.bytes_read >= self.file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_async_file_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test file.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = AsyncFileReader::open(temp_file.path()).await.unwrap();
        assert_eq!(reader.file_size(), test_data.len() as u64);
        assert_eq!(reader.bytes_read(), 0);
        assert!(!reader.is_eof());

        let mut buffer = AdaptiveBuffer::new(ferrocp_types::DeviceType::SSD);
        let bytes_read = reader.read_into_buffer(&mut buffer).await.unwrap();
        
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(reader.bytes_read(), test_data.len() as u64);
        assert!(reader.is_eof());
        assert_eq!(buffer.as_ref(), test_data);
    }

    #[test]
    fn test_sync_file_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test file.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut reader = FileReader::open(temp_file.path()).unwrap();
        assert_eq!(reader.file_size(), test_data.len() as u64);
        assert_eq!(reader.bytes_read(), 0);
        assert!(!reader.is_eof());

        let mut buffer = vec![0u8; test_data.len()];
        let bytes_read = reader.read(&mut buffer).unwrap();
        
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(reader.bytes_read(), test_data.len() as u64);
        assert!(reader.is_eof());
        assert_eq!(buffer, test_data);
    }
}
