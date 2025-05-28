//! Smart buffering system for optimal I/O performance

use ferrocp_types::{BufferSize, DeviceType, Result};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use bytes::{Bytes, BytesMut};

/// Adaptive buffer that adjusts size based on performance characteristics
#[derive(Debug)]
pub struct AdaptiveBuffer {
    buffer: BytesMut,
    optimal_size: usize,
    min_size: usize,
    max_size: usize,
    device_type: DeviceType,
}

impl AdaptiveBuffer {
    /// Create a new adaptive buffer
    pub fn new(device_type: DeviceType) -> Self {
        let (min_size, optimal_size, max_size) = Self::get_size_limits(device_type);
        
        Self {
            buffer: BytesMut::with_capacity(optimal_size),
            optimal_size,
            min_size,
            max_size,
            device_type,
        }
    }

    /// Create a new adaptive buffer with custom size
    pub fn with_size(device_type: DeviceType, size: usize) -> Self {
        let (min_size, _, max_size) = Self::get_size_limits(device_type);
        let optimal_size = size.clamp(min_size, max_size);
        
        Self {
            buffer: BytesMut::with_capacity(optimal_size),
            optimal_size,
            min_size,
            max_size,
            device_type,
        }
    }

    /// Get the current buffer capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get the current buffer length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get a mutable reference to the buffer
    pub fn as_mut(&mut self) -> &mut BytesMut {
        &mut self.buffer
    }

    /// Get an immutable reference to the buffer
    pub fn as_ref(&self) -> &[u8] {
        &self.buffer
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        let new_capacity = (self.buffer.capacity() + additional).min(self.max_size);
        if new_capacity > self.buffer.capacity() {
            self.buffer.reserve(new_capacity - self.buffer.capacity());
        }
    }

    /// Resize the buffer based on performance feedback
    pub fn adapt_size(&mut self, throughput_mbps: f64, latency_ms: f64) {
        let new_size = if throughput_mbps < 50.0 || latency_ms > 100.0 {
            // Poor performance, try smaller buffer
            (self.optimal_size / 2).max(self.min_size)
        } else if throughput_mbps > 200.0 && latency_ms < 10.0 {
            // Good performance, try larger buffer
            (self.optimal_size * 2).min(self.max_size)
        } else {
            self.optimal_size
        };

        if new_size != self.optimal_size {
            self.optimal_size = new_size;
            self.buffer.reserve(new_size.saturating_sub(self.buffer.capacity()));
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Split the buffer at the given index
    pub fn split_to(&mut self, at: usize) -> Bytes {
        self.buffer.split_to(at).freeze()
    }

    /// Get size limits for different device types
    fn get_size_limits(device_type: DeviceType) -> (usize, usize, usize) {
        match device_type {
            DeviceType::SSD => (64 * 1024, 1024 * 1024, 16 * 1024 * 1024), // 64KB - 16MB
            DeviceType::HDD => (4 * 1024, 64 * 1024, 1024 * 1024),         // 4KB - 1MB
            DeviceType::Network => (8 * 1024, 128 * 1024, 2 * 1024 * 1024), // 8KB - 2MB
            DeviceType::RamDisk => (1024 * 1024, 8 * 1024 * 1024, 64 * 1024 * 1024), // 1MB - 64MB
            DeviceType::Unknown => (8 * 1024, 256 * 1024, 4 * 1024 * 1024), // 8KB - 4MB
        }
    }
}

/// Smart buffer that automatically manages memory allocation
#[derive(Debug)]
pub struct SmartBuffer {
    inner: AdaptiveBuffer,
    usage_stats: BufferUsageStats,
}

/// Statistics for buffer usage tracking
#[derive(Debug, Default)]
pub struct BufferUsageStats {
    /// Total number of read operations
    pub total_reads: u64,
    /// Total number of write operations
    pub total_writes: u64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Average operation size in bytes
    pub avg_operation_size: f64,
}

impl SmartBuffer {
    /// Create a new smart buffer
    pub fn new(device_type: DeviceType) -> Self {
        Self {
            inner: AdaptiveBuffer::new(device_type),
            usage_stats: BufferUsageStats::default(),
        }
    }

    /// Record a read operation
    pub fn record_read(&mut self, bytes: usize) {
        self.usage_stats.total_reads += 1;
        self.usage_stats.total_bytes += bytes as u64;
        self.update_avg_operation_size();
    }

    /// Record a write operation
    pub fn record_write(&mut self, bytes: usize) {
        self.usage_stats.total_writes += 1;
        self.usage_stats.total_bytes += bytes as u64;
        self.update_avg_operation_size();
    }

    /// Get buffer statistics
    pub fn stats(&self) -> &BufferUsageStats {
        &self.usage_stats
    }

    /// Get the underlying adaptive buffer
    pub fn inner(&mut self) -> &mut AdaptiveBuffer {
        &mut self.inner
    }

    fn update_avg_operation_size(&mut self) {
        let total_ops = self.usage_stats.total_reads + self.usage_stats.total_writes;
        if total_ops > 0 {
            self.usage_stats.avg_operation_size = 
                self.usage_stats.total_bytes as f64 / total_ops as f64;
        }
    }
}

/// Buffer pool for reusing buffers to reduce allocations
#[derive(Debug)]
pub struct BufferPool {
    buffers: Arc<Mutex<VecDeque<BytesMut>>>,
    buffer_size: usize,
    max_pool_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size,
            max_pool_size,
        }
    }

    /// Get a buffer from the pool or create a new one
    pub fn get_buffer(&self) -> BytesMut {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.pop_front().unwrap_or_else(|| BytesMut::with_capacity(self.buffer_size))
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&self, mut buffer: BytesMut) {
        buffer.clear();
        
        let mut buffers = self.buffers.lock().unwrap();
        if buffers.len() < self.max_pool_size {
            buffers.push_back(buffer);
        }
        // If pool is full, just drop the buffer
    }

    /// Get the current pool size
    pub fn pool_size(&self) -> usize {
        self.buffers.lock().unwrap().len()
    }

    /// Clear all buffers from the pool
    pub fn clear(&self) {
        self.buffers.lock().unwrap().clear();
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new(64 * 1024, 16) // 64KB buffers, max 16 in pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_buffer_creation() {
        let buffer = AdaptiveBuffer::new(DeviceType::SSD);
        assert!(buffer.capacity() >= 64 * 1024);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_adaptive_buffer_size_adaptation() {
        let mut buffer = AdaptiveBuffer::new(DeviceType::SSD);
        let initial_size = buffer.optimal_size;
        
        // Simulate poor performance
        buffer.adapt_size(30.0, 150.0);
        assert!(buffer.optimal_size <= initial_size);
        
        // Simulate good performance
        buffer.adapt_size(300.0, 5.0);
        assert!(buffer.optimal_size >= initial_size);
    }

    #[test]
    fn test_smart_buffer_stats() {
        let mut buffer = SmartBuffer::new(DeviceType::SSD);
        
        buffer.record_read(1024);
        buffer.record_write(2048);
        
        let stats = buffer.stats();
        assert_eq!(stats.total_reads, 1);
        assert_eq!(stats.total_writes, 1);
        assert_eq!(stats.total_bytes, 3072);
        assert_eq!(stats.avg_operation_size, 1536.0);
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(1024, 5);
        
        let buffer1 = pool.get_buffer();
        let buffer2 = pool.get_buffer();
        
        assert_eq!(buffer1.capacity(), 1024);
        assert_eq!(buffer2.capacity(), 1024);
        
        pool.return_buffer(buffer1);
        assert_eq!(pool.pool_size(), 1);
        
        pool.return_buffer(buffer2);
        assert_eq!(pool.pool_size(), 2);
    }
}
