//! Async file writer with smart buffering

use crate::AdaptiveBuffer;
use ferrocp_types::{Error, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::debug;

/// Async file writer with adaptive buffering
#[derive(Debug)]
pub struct AsyncFileWriter {
    writer: BufWriter<File>,
    bytes_written: u64,
}

impl AsyncFileWriter {
    /// Create a new file for writing
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::create(path).await.map_err(|e| Error::Io {
            message: format!("Failed to create file '{}': {}", path.display(), e),
        })?;

        let writer = BufWriter::new(file);

        debug!("Created file for writing: {}", path.display());

        Ok(Self {
            writer,
            bytes_written: 0,
        })
    }

    /// Open an existing file for writing (truncates the file)
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::options()
            .write(true)
            .truncate(true)
            .open(path)
            .await
            .map_err(|e| Error::Io {
                message: format!("Failed to open file '{}': {}", path.display(), e),
            })?;

        let writer = BufWriter::new(file);

        debug!("Opened file for writing: {}", path.display());

        Ok(Self {
            writer,
            bytes_written: 0,
        })
    }

    /// Write data from an adaptive buffer
    pub async fn write_from_buffer(
        &mut self,
        buffer: &AdaptiveBuffer,
        len: usize,
    ) -> Result<usize> {
        let data = &buffer.as_ref()[..len];
        let bytes_written = self.writer.write(data).await.map_err(|e| Error::Io {
            message: format!("Failed to write to file: {}", e),
        })?;

        self.bytes_written += bytes_written as u64;

        debug!(
            "Wrote {} bytes to file ({} total)",
            bytes_written, self.bytes_written
        );
        Ok(bytes_written)
    }

    /// Write data from a byte slice
    pub async fn write(&mut self, data: &[u8]) -> Result<usize> {
        let bytes_written = self.writer.write(data).await.map_err(|e| Error::Io {
            message: format!("Failed to write to file: {}", e),
        })?;

        self.bytes_written += bytes_written as u64;
        Ok(bytes_written)
    }

    /// Write all data from a byte slice
    pub async fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).await.map_err(|e| Error::Io {
            message: format!("Failed to write all data: {}", e),
        })?;

        self.bytes_written += data.len() as u64;
        Ok(())
    }

    /// Flush the writer to ensure all data is written
    pub async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await.map_err(|e| Error::Io {
            message: format!("Failed to flush writer: {}", e),
        })?;

        debug!("Flushed writer");
        Ok(())
    }

    /// Sync all data to disk
    pub async fn sync_all(&mut self) -> Result<()> {
        self.writer
            .get_mut()
            .sync_all()
            .await
            .map_err(|e| Error::Io {
                message: format!("Failed to sync to disk: {}", e),
            })?;

        debug!("Synced all data to disk");
        Ok(())
    }

    /// Get the number of bytes written so far
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

/// Synchronous file writer for compatibility
#[derive(Debug)]
pub struct FileWriter {
    writer: std::io::BufWriter<std::fs::File>,
    bytes_written: u64,
}

impl FileWriter {
    /// Create a new file for writing
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::create(path).map_err(|e| Error::Io {
            message: format!("Failed to create file '{}': {}", path.display(), e),
        })?;

        let writer = std::io::BufWriter::new(file);

        debug!("Created file for writing: {}", path.display());

        Ok(Self {
            writer,
            bytes_written: 0,
        })
    }

    /// Open an existing file for writing (truncates the file)
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::options()
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| Error::Io {
                message: format!("Failed to open file '{}': {}", path.display(), e),
            })?;

        let writer = std::io::BufWriter::new(file);

        debug!("Opened file for writing: {}", path.display());

        Ok(Self {
            writer,
            bytes_written: 0,
        })
    }

    /// Write data from a byte slice
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        use std::io::Write;

        let bytes_written = self.writer.write(data).map_err(|e| Error::Io {
            message: format!("Failed to write to file: {}", e),
        })?;

        self.bytes_written += bytes_written as u64;
        Ok(bytes_written)
    }

    /// Write all data from a byte slice
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;

        self.writer.write_all(data).map_err(|e| Error::Io {
            message: format!("Failed to write all data: {}", e),
        })?;

        self.bytes_written += data.len() as u64;
        Ok(())
    }

    /// Flush the writer to ensure all data is written
    pub fn flush(&mut self) -> Result<()> {
        use std::io::Write;

        self.writer.flush().map_err(|e| Error::Io {
            message: format!("Failed to flush writer: {}", e),
        })?;

        debug!("Flushed writer");
        Ok(())
    }

    /// Sync all data to disk
    pub fn sync_all(&mut self) -> Result<()> {
        self.writer.get_mut().sync_all().map_err(|e| Error::Io {
            message: format!("Failed to sync to disk: {}", e),
        })?;

        debug!("Synced all data to disk");
        Ok(())
    }

    /// Get the number of bytes written so far
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_async_file_writer() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World! This is a test file.";

        let mut writer = AsyncFileWriter::create(&file_path).await.unwrap();
        assert_eq!(writer.bytes_written(), 0);

        writer.write_all(test_data).await.unwrap();
        assert_eq!(writer.bytes_written(), test_data.len() as u64);

        writer.flush().await.unwrap();
        writer.sync_all().await.unwrap();

        // Verify the file was written correctly
        let written_data = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(written_data, test_data);
    }

    #[test]
    fn test_sync_file_writer() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World! This is a test file.";

        let mut writer = FileWriter::create(&file_path).unwrap();
        assert_eq!(writer.bytes_written(), 0);

        writer.write_all(test_data).unwrap();
        assert_eq!(writer.bytes_written(), test_data.len() as u64);

        writer.flush().unwrap();
        writer.sync_all().unwrap();

        // Verify the file was written correctly
        let written_data = std::fs::read(&file_path).unwrap();
        assert_eq!(written_data, test_data);
    }

    #[tokio::test]
    async fn test_write_from_buffer() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World!";

        let mut buffer = AdaptiveBuffer::new(ferrocp_types::DeviceType::SSD);
        buffer.as_mut().extend_from_slice(test_data);

        let mut writer = AsyncFileWriter::create(&file_path).await.unwrap();
        let bytes_written = writer
            .write_from_buffer(&buffer, test_data.len())
            .await
            .unwrap();

        assert_eq!(bytes_written, test_data.len());
        assert_eq!(writer.bytes_written(), test_data.len() as u64);

        writer.flush().await.unwrap();

        // Verify the file was written correctly
        let written_data = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(written_data, test_data);
    }
}
