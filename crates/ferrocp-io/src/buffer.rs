//! Smart buffering system for optimal I/O performance

use bytes::{Bytes, BytesMut};
use ferrocp_types::DeviceType;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

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
        let current_capacity = self.buffer.capacity();
        let new_capacity = current_capacity.saturating_add(additional).min(self.max_size);
        if new_capacity > current_capacity {
            self.buffer.reserve(new_capacity - current_capacity);
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
            self.buffer
                .reserve(new_size.saturating_sub(self.buffer.capacity()));
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

    /// Get the device type this buffer is optimized for
    pub fn device_type(&self) -> DeviceType {
        self.device_type
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

/// Smart buffer that automatically manages memory allocation with multi-size pool support
#[derive(Debug)]
pub struct SmartBuffer {
    inner: AdaptiveBuffer,
    usage_stats: BufferUsageStats,
    pool: Option<Arc<MultiSizeBufferPool>>,
    memory_monitor: Option<Arc<crate::memory::MemoryMonitor>>,
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
            pool: None,
            memory_monitor: None,
        }
    }

    /// Create a new smart buffer with multi-size pool support
    pub fn with_pool(device_type: DeviceType, pool: Arc<MultiSizeBufferPool>) -> Self {
        Self {
            inner: AdaptiveBuffer::new(device_type),
            usage_stats: BufferUsageStats::default(),
            pool: Some(pool),
            memory_monitor: None,
        }
    }

    /// Create a new smart buffer with memory monitoring
    pub fn with_monitoring(
        device_type: DeviceType,
        pool: Arc<MultiSizeBufferPool>,
        monitor: Arc<crate::memory::MemoryMonitor>,
    ) -> Self {
        Self {
            inner: AdaptiveBuffer::new(device_type),
            usage_stats: BufferUsageStats::default(),
            pool: Some(pool),
            memory_monitor: Some(monitor),
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

    /// Get a buffer from the pool if available, otherwise use the adaptive buffer
    pub fn get_pooled_buffer(&self, size: usize) -> Option<BytesMut> {
        self.pool.as_ref().map(|pool| pool.get_buffer(size))
    }

    /// Return a buffer to the pool
    pub fn return_pooled_buffer(&self, buffer: BytesMut) {
        if let Some(pool) = &self.pool {
            pool.return_buffer(buffer);
        }
    }

    /// Get current memory statistics from the pool
    pub fn memory_stats(&self) -> Option<MemoryStats> {
        self.pool.as_ref().map(|pool| pool.memory_stats())
    }

    /// Check memory status and get alerts
    pub fn check_memory_status(&self) -> Option<crate::memory::MemoryAlert> {
        if let (Some(pool), Some(monitor)) = (&self.pool, &self.memory_monitor) {
            let stats = pool.memory_stats();
            Some(monitor.check_memory_status(stats.current_used, stats.efficiency()))
        } else {
            None
        }
    }

    /// Force cleanup of unused buffers
    pub fn cleanup_buffers(&self) {
        if let Some(pool) = &self.pool {
            pool.cleanup_unused_buffers();
        }
    }

    fn update_avg_operation_size(&mut self) {
        let total_ops = self.usage_stats.total_reads + self.usage_stats.total_writes;
        if total_ops > 0 {
            self.usage_stats.avg_operation_size =
                self.usage_stats.total_bytes as f64 / total_ops as f64;
        }

        // Update memory monitor if available
        if let (Some(pool), Some(monitor)) = (&self.pool, &self.memory_monitor) {
            let stats = pool.memory_stats();
            monitor.record_usage(stats.current_used, stats.active_buffers, stats.efficiency());
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
        buffers
            .pop_front()
            .unwrap_or_else(|| BytesMut::with_capacity(self.buffer_size))
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

/// Memory usage statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total allocated memory in bytes
    pub total_allocated: u64,
    /// Currently used memory in bytes
    pub current_used: u64,
    /// Peak memory usage in bytes
    pub peak_usage: u64,
    /// Number of active buffers
    pub active_buffers: u64,
    /// Number of pooled buffers
    pub pooled_buffers: u64,
    /// Memory allocation count
    pub allocation_count: u64,
    /// Memory deallocation count
    pub deallocation_count: u64,
}

impl MemoryStats {
    /// Calculate memory efficiency as a percentage
    pub fn efficiency(&self) -> f64 {
        if self.total_allocated == 0 {
            return 100.0;
        }
        (self.current_used as f64 / self.total_allocated as f64) * 100.0
    }

    /// Check if memory usage is within acceptable limits
    pub fn is_healthy(&self, max_memory_mb: u64) -> bool {
        let max_bytes = max_memory_mb * 1024 * 1024;
        self.current_used <= max_bytes && self.efficiency() >= 60.0
    }
}

/// Multi-size buffer pool that manages different buffer sizes efficiently
#[derive(Debug)]
pub struct MultiSizeBufferPool {
    pools: HashMap<usize, BufferPool>,
    memory_stats: Arc<Mutex<MemoryStats>>,
    max_memory_mb: u64,
    auto_cleanup_threshold: f64,
}

impl MultiSizeBufferPool {
    /// Create a new multi-size buffer pool
    pub fn new(max_memory_mb: u64) -> Self {
        let mut pools = HashMap::new();

        // Standard buffer sizes: 4KB, 64KB, 1MB, 4MB
        let sizes = [4 * 1024, 64 * 1024, 1024 * 1024, 4 * 1024 * 1024];
        let max_pool_sizes = [32, 16, 8, 4]; // Fewer large buffers

        for (&size, &max_pool_size) in sizes.iter().zip(max_pool_sizes.iter()) {
            pools.insert(size, BufferPool::new(size, max_pool_size));
        }

        Self {
            pools,
            memory_stats: Arc::new(Mutex::new(MemoryStats::default())),
            max_memory_mb,
            auto_cleanup_threshold: 80.0, // Cleanup when 80% memory used
        }
    }

    /// Get a buffer of the specified size or the next larger available size
    pub fn get_buffer(&self, requested_size: usize) -> BytesMut {
        let optimal_size = self.find_optimal_size(requested_size);

        let buffer = if let Some(pool) = self.pools.get(&optimal_size) {
            pool.get_buffer()
        } else {
            // Fallback to creating a new buffer
            BytesMut::with_capacity(requested_size)
        };

        // Update memory statistics
        self.update_allocation_stats(buffer.capacity());

        // Check if cleanup is needed
        if self.should_cleanup() {
            self.cleanup_unused_buffers();
        }

        buffer
    }

    /// Return a buffer to the appropriate pool
    pub fn return_buffer(&self, buffer: BytesMut) {
        let size = buffer.capacity();

        if let Some(pool) = self.pools.get(&size) {
            pool.return_buffer(buffer);
            self.update_deallocation_stats(size);
        }
        // If no matching pool, buffer is dropped automatically
    }

    /// Get current memory statistics
    pub fn memory_stats(&self) -> MemoryStats {
        self.memory_stats.lock().unwrap().clone()
    }

    /// Force cleanup of unused buffers
    pub fn cleanup_unused_buffers(&self) {
        for pool in self.pools.values() {
            // Clear half of the buffers in each pool to free memory
            let current_size = pool.pool_size();
            let target_size = current_size / 2;

            for _ in target_size..current_size {
                if pool.pool_size() > target_size {
                    let _ = pool.get_buffer(); // Get and drop buffer
                }
            }
        }

        // Update memory stats after cleanup
        self.recalculate_memory_stats();
    }

    /// Find the optimal buffer size for the requested size
    fn find_optimal_size(&self, requested_size: usize) -> usize {
        let sizes = [4 * 1024, 64 * 1024, 1024 * 1024, 4 * 1024 * 1024];

        for &size in &sizes {
            if requested_size <= size {
                return size;
            }
        }

        // If larger than all predefined sizes, use the largest
        sizes[sizes.len() - 1]
    }

    /// Check if cleanup should be performed
    fn should_cleanup(&self) -> bool {
        let stats = self.memory_stats.lock().unwrap();
        let usage_percentage = (stats.current_used as f64 / (self.max_memory_mb * 1024 * 1024) as f64) * 100.0;
        usage_percentage > self.auto_cleanup_threshold
    }

    /// Update allocation statistics
    fn update_allocation_stats(&self, size: usize) {
        let mut stats = self.memory_stats.lock().unwrap();
        stats.allocation_count += 1;
        stats.active_buffers += 1;
        stats.current_used += size as u64;
        stats.total_allocated += size as u64;

        if stats.current_used > stats.peak_usage {
            stats.peak_usage = stats.current_used;
        }
    }

    /// Update deallocation statistics
    fn update_deallocation_stats(&self, size: usize) {
        let mut stats = self.memory_stats.lock().unwrap();
        stats.deallocation_count += 1;
        stats.active_buffers = stats.active_buffers.saturating_sub(1);
        stats.current_used = stats.current_used.saturating_sub(size as u64);
        stats.pooled_buffers += 1;
    }

    /// Recalculate memory statistics after cleanup
    fn recalculate_memory_stats(&self) {
        let mut stats = self.memory_stats.lock().unwrap();
        let mut total_pooled = 0;

        for (size, pool) in &self.pools {
            total_pooled += pool.pool_size() as u64 * (*size as u64);
        }

        stats.pooled_buffers = total_pooled / 1024; // Convert to KB for easier reading
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

    #[test]
    fn test_multi_size_buffer_pool() {
        let pool = MultiSizeBufferPool::new(64); // 64MB max

        // Test different buffer sizes
        let small_buffer = pool.get_buffer(2048); // Should get 4KB buffer
        assert!(small_buffer.capacity() >= 2048);

        let medium_buffer = pool.get_buffer(32 * 1024); // Should get 64KB buffer
        assert!(medium_buffer.capacity() >= 32 * 1024);

        let large_buffer = pool.get_buffer(512 * 1024); // Should get 1MB buffer
        assert!(large_buffer.capacity() >= 512 * 1024);

        // Return buffers
        pool.return_buffer(small_buffer);
        pool.return_buffer(medium_buffer);
        pool.return_buffer(large_buffer);

        // Check memory stats
        let stats = pool.memory_stats();
        assert!(stats.pooled_buffers > 0);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            total_allocated: 1024 * 1024,
            current_used: 512 * 1024,
            peak_usage: 768 * 1024,
            active_buffers: 10,
            pooled_buffers: 5,
            allocation_count: 15,
            deallocation_count: 5,
        };

        assert_eq!(stats.efficiency(), 50.0);
        // 512KB used, 2MB limit = 25% usage, but efficiency is 50% < 60% threshold
        assert!(!stats.is_healthy(2)); // Should be unhealthy due to low efficiency
        assert!(!stats.is_healthy(1)); // 1MB limit (too small)

        // Test with good efficiency
        let healthy_stats = MemoryStats {
            total_allocated: 1024 * 1024,
            current_used: 700 * 1024, // 70% efficiency
            peak_usage: 768 * 1024,
            active_buffers: 10,
            pooled_buffers: 5,
            allocation_count: 15,
            deallocation_count: 5,
        };
        assert!(healthy_stats.is_healthy(2)); // Should be healthy
    }

    #[test]
    fn test_smart_buffer_with_pool() {
        let pool = Arc::new(MultiSizeBufferPool::new(32)); // 32MB max
        let buffer = SmartBuffer::with_pool(DeviceType::SSD, pool.clone());

        // Test pooled buffer operations
        if let Some(pooled_buffer) = buffer.get_pooled_buffer(8192) {
            assert!(pooled_buffer.capacity() >= 8192);
            buffer.return_pooled_buffer(pooled_buffer);
        }

        // Test memory stats
        assert!(buffer.memory_stats().is_some());
    }
}
