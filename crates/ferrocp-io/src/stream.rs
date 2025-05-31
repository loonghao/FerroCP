//! Streaming utilities for file operations

use bytes::Bytes;
use ferrocp_types::{Error, ProgressInfo, Result};
use futures::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

/// File stream for reading files in chunks
#[derive(Debug)]
pub struct FileStream {
    file: File,
    chunk_size: usize,
    file_size: u64,
    bytes_read: u64,
    buffer: Vec<u8>,
}

impl FileStream {
    /// Create a new file stream
    pub async fn new<P: Into<PathBuf>>(path: P, chunk_size: usize) -> Result<Self> {
        let path = path.into();
        let file = File::open(&path).await.map_err(|e| Error::Io {
            message: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        let metadata = file.metadata().await.map_err(|e| Error::Io {
            message: format!("Failed to read file metadata: {}", e),
        })?;

        let file_size = metadata.len();
        let buffer = vec![0u8; chunk_size];

        Ok(Self {
            file,
            chunk_size,
            file_size,
            bytes_read: 0,
            buffer,
        })
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

    /// Get the chunk size used for reading
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.bytes_read >= self.file_size {
            return Poll::Ready(None);
        }

        // Split the mutable borrow to avoid conflicts
        let this = self.as_mut().get_mut();
        let mut read_buf = ReadBuf::new(&mut this.buffer);

        match Pin::new(&mut this.file).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let bytes_read = read_buf.filled().len();
                if bytes_read == 0 {
                    Poll::Ready(None)
                } else {
                    this.bytes_read += bytes_read as u64;
                    let data = Bytes::copy_from_slice(&read_buf.filled()[..bytes_read]);
                    Poll::Ready(Some(Ok(data)))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(Error::Io {
                message: format!("Failed to read from file: {}", e),
            }))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Progress-aware stream wrapper
pub struct ProgressStream<S> {
    inner: S,
    file_path: PathBuf,
    file_size: u64,
    bytes_processed: u64,
    progress_callback: Option<Box<dyn Fn(ProgressInfo) + Send + Sync>>,
}

impl<S> std::fmt::Debug for ProgressStream<S>
where
    S: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressStream")
            .field("inner", &self.inner)
            .field("file_path", &self.file_path)
            .field("file_size", &self.file_size)
            .field("bytes_processed", &self.bytes_processed)
            .field("progress_callback", &"<callback>")
            .finish()
    }
}

impl<S> ProgressStream<S> {
    /// Create a new progress stream
    pub fn new<P: Into<PathBuf>>(inner: S, file_path: P, file_size: u64) -> Self {
        Self {
            inner,
            file_path: file_path.into(),
            file_size,
            bytes_processed: 0,
            progress_callback: None,
        }
    }

    /// Set a progress callback
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProgressInfo) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// Get the current progress
    pub fn progress(&self) -> f64 {
        if self.file_size > 0 {
            (self.bytes_processed as f64 / self.file_size as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get bytes processed
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }

    fn report_progress(&self) {
        if let Some(ref callback) = self.progress_callback {
            let progress = ProgressInfo {
                current_file: self.file_path.clone(),
                current_file_bytes: self.bytes_processed,
                current_file_size: self.file_size,
                files_processed: 1,
                total_files: 1,
                bytes_processed: self.bytes_processed,
                total_bytes: self.file_size,
                transfer_rate: 0.0, // Would need timing info
                eta: None,
            };
            callback(progress);
        }
    }
}

impl<S> Stream for ProgressStream<S>
where
    S: Stream<Item = Result<Bytes>> + Unpin,
{
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.bytes_processed += bytes.len() as u64;
                self.report_progress();
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                self.report_progress(); // Final progress report
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Utility functions for stream operations
pub mod utils {
    use super::*;
    use futures::TryStreamExt;
    use tokio::io::AsyncWriteExt;

    /// Copy data from a stream to a writer
    pub async fn copy_stream_to_writer<S, W>(mut stream: S, mut writer: W) -> Result<u64>
    where
        S: Stream<Item = Result<Bytes>> + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut total_bytes = 0u64;

        while let Some(chunk) = stream.try_next().await? {
            use tokio::io::AsyncWriteExt;
            writer.write_all(&chunk).await.map_err(|e| Error::Io {
                message: format!("Failed to write chunk: {}", e),
            })?;
            total_bytes += chunk.len() as u64;
        }

        writer.flush().await.map_err(|e| Error::Io {
            message: format!("Failed to flush writer: {}", e),
        })?;

        Ok(total_bytes)
    }

    /// Collect all chunks from a stream into a single buffer
    pub async fn collect_stream<S>(stream: S) -> Result<Vec<u8>>
    where
        S: Stream<Item = Result<Bytes>> + Unpin,
    {
        let chunks: Vec<Bytes> = stream.try_collect().await?;
        let total_size: usize = chunks.iter().map(|chunk| chunk.len()).sum();

        let mut buffer = Vec::with_capacity(total_size);
        for chunk in chunks {
            buffer.extend_from_slice(&chunk);
        }

        Ok(buffer)
    }

    /// Count the total bytes in a stream without collecting them
    pub async fn count_stream_bytes<S>(stream: S) -> Result<u64>
    where
        S: Stream<Item = Result<Bytes>> + Unpin,
    {
        let mut total_bytes = 0u64;
        let mut stream = stream;

        while let Some(chunk) = stream.try_next().await? {
            total_bytes += chunk.len() as u64;
        }

        Ok(total_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_stream() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test file for streaming.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let mut stream = FileStream::new(temp_file.path(), 10).await.unwrap();
        assert_eq!(stream.file_size(), test_data.len() as u64);

        let mut collected_data = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            collected_data.extend_from_slice(&chunk);
        }

        assert_eq!(collected_data, test_data);
        assert_eq!(stream.bytes_read(), test_data.len() as u64);
        assert_eq!(stream.progress(), 100.0);
    }

    #[tokio::test]
    async fn test_progress_stream() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let file_stream = FileStream::new(temp_file.path(), 5).await.unwrap();
        let mut progress_stream =
            ProgressStream::new(file_stream, temp_file.path(), test_data.len() as u64);

        let mut total_bytes = 0;
        while let Some(chunk) = progress_stream.next().await {
            let chunk = chunk.unwrap();
            total_bytes += chunk.len();
        }

        assert_eq!(total_bytes, test_data.len());
        assert_eq!(progress_stream.bytes_processed(), test_data.len() as u64);
        assert_eq!(progress_stream.progress(), 100.0);
    }

    #[tokio::test]
    async fn test_stream_utils() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World! This is a test.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let stream = FileStream::new(temp_file.path(), 10).await.unwrap();

        // Test collecting stream
        let collected = utils::collect_stream(stream).await.unwrap();
        assert_eq!(collected, test_data);

        // Test counting bytes
        let stream = FileStream::new(temp_file.path(), 10).await.unwrap();
        let byte_count = utils::count_stream_bytes(stream).await.unwrap();
        assert_eq!(byte_count, test_data.len() as u64);
    }
}
