//! Property-based tests for ferrocp-io components

use crate::buffer::{AdaptiveBuffer, BufferPool, SmartBuffer};
use crate::memory_map::MemoryMappedFile;
use ferrocp_types::DeviceType;
use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

/// Generate arbitrary device types for testing
fn device_type_strategy() -> impl Strategy<Value = DeviceType> {
    prop_oneof![
        Just(DeviceType::SSD),
        Just(DeviceType::HDD),
        Just(DeviceType::Network),
        Just(DeviceType::RamDisk),
        Just(DeviceType::Unknown),
    ]
}

/// Generate reasonable buffer sizes (1KB to 64MB)
fn buffer_size_strategy() -> impl Strategy<Value = usize> {
    (1024usize..=64 * 1024 * 1024).prop_filter("Must be power of 2 or reasonable size", |&size| {
        size >= 1024 && size <= 64 * 1024 * 1024
    })
}

/// Generate performance metrics for buffer adaptation
fn performance_metrics_strategy() -> impl Strategy<Value = (f64, f64)> {
    (0.0f64..=1000.0, 0.0f64..=1000.0)
}

proptest! {
    /// Test that AdaptiveBuffer always maintains valid size constraints
    #[test]
    fn test_adaptive_buffer_size_constraints(
        device_type in device_type_strategy(),
        custom_size in buffer_size_strategy()
    ) {
        let buffer = AdaptiveBuffer::with_size(device_type, custom_size);
        
        // Buffer capacity should be within device-specific limits
        let (min_size, _, max_size) = match device_type {
            DeviceType::SSD => (64 * 1024, 1024 * 1024, 16 * 1024 * 1024),
            DeviceType::HDD => (4 * 1024, 64 * 1024, 1024 * 1024),
            DeviceType::Network => (8 * 1024, 128 * 1024, 2 * 1024 * 1024),
            DeviceType::RamDisk => (1024 * 1024, 8 * 1024 * 1024, 64 * 1024 * 1024),
            DeviceType::Unknown => (8 * 1024, 256 * 1024, 4 * 1024 * 1024),
        };
        
        prop_assert!(buffer.capacity() >= min_size);
        prop_assert!(buffer.capacity() <= max_size);
        prop_assert!(buffer.is_empty());
    }

    /// Test buffer adaptation behavior with various performance metrics
    #[test]
    fn test_buffer_adaptation_properties(
        device_type in device_type_strategy(),
        (throughput, latency) in performance_metrics_strategy()
    ) {
        let mut buffer = AdaptiveBuffer::new(device_type);
        let initial_size = buffer.capacity();
        
        buffer.adapt_size(throughput, latency);
        
        // Buffer size should always be within valid bounds
        let (min_size, _, max_size) = match device_type {
            DeviceType::SSD => (64 * 1024, 1024 * 1024, 16 * 1024 * 1024),
            DeviceType::HDD => (4 * 1024, 64 * 1024, 1024 * 1024),
            DeviceType::Network => (8 * 1024, 128 * 1024, 2 * 1024 * 1024),
            DeviceType::RamDisk => (1024 * 1024, 8 * 1024 * 1024, 64 * 1024 * 1024),
            DeviceType::Unknown => (8 * 1024, 256 * 1024, 4 * 1024 * 1024),
        };
        
        prop_assert!(buffer.capacity() >= min_size);
        prop_assert!(buffer.capacity() <= max_size);
        
        // Poor performance should not increase buffer size
        if throughput < 50.0 || latency > 100.0 {
            prop_assert!(buffer.capacity() <= initial_size);
        }
        
        // Good performance should not decrease buffer size below initial
        if throughput > 200.0 && latency < 10.0 {
            prop_assert!(buffer.capacity() >= initial_size);
        }
    }

    /// Test SmartBuffer statistics consistency
    #[test]
    fn test_smart_buffer_statistics(
        device_type in device_type_strategy(),
        read_operations in prop::collection::vec(1usize..=1024*1024, 0..=100),
        write_operations in prop::collection::vec(1usize..=1024*1024, 0..=100)
    ) {
        let mut buffer = SmartBuffer::new(device_type);
        
        let mut expected_total_bytes = 0u64;
        let mut expected_reads = 0u64;
        let mut expected_writes = 0u64;
        
        for &read_size in &read_operations {
            buffer.record_read(read_size);
            expected_total_bytes += read_size as u64;
            expected_reads += 1;
        }
        
        for &write_size in &write_operations {
            buffer.record_write(write_size);
            expected_total_bytes += write_size as u64;
            expected_writes += 1;
        }
        
        let stats = buffer.stats();
        prop_assert_eq!(stats.total_reads, expected_reads);
        prop_assert_eq!(stats.total_writes, expected_writes);
        prop_assert_eq!(stats.total_bytes, expected_total_bytes);
        
        let total_ops = expected_reads + expected_writes;
        if total_ops > 0 {
            let expected_avg = expected_total_bytes as f64 / total_ops as f64;
            prop_assert!((stats.avg_operation_size - expected_avg).abs() < 0.001);
        }
    }

    /// Test BufferPool behavior under various conditions
    #[test]
    fn test_buffer_pool_properties(
        buffer_size in 1024usize..=1024*1024,
        max_pool_size in 1usize..=100,
        operations in prop::collection::vec(prop::bool::ANY, 0..=200)
    ) {
        let pool = BufferPool::new(buffer_size, max_pool_size);
        let mut borrowed_buffers = Vec::new();
        
        for &should_get in &operations {
            if should_get && borrowed_buffers.len() < max_pool_size * 2 {
                // Get a buffer
                let buffer = pool.get_buffer();
                prop_assert_eq!(buffer.capacity(), buffer_size);
                borrowed_buffers.push(buffer);
            } else if !borrowed_buffers.is_empty() {
                // Return a buffer
                let buffer = borrowed_buffers.pop().unwrap();
                pool.return_buffer(buffer);
            }
            
            // Pool size should never exceed max_pool_size
            prop_assert!(pool.pool_size() <= max_pool_size);
        }
        
        // Return all remaining buffers
        for buffer in borrowed_buffers {
            pool.return_buffer(buffer);
        }
        
        // Final pool size should not exceed max_pool_size
        prop_assert!(pool.pool_size() <= max_pool_size);
    }
}

/// Fuzz test for memory-mapped file operations
#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use std::io::Write;

    proptest! {
        /// Fuzz test memory-mapped file with various data patterns
        #[test]
        fn fuzz_memory_mapped_file(
            data in prop::collection::vec(any::<u8>(), 0..=1024*1024)
        ) {
            if data.is_empty() {
                return Ok(());
            }
            
            let mut temp_file = NamedTempFile::new().unwrap();
            temp_file.write_all(&data).unwrap();
            temp_file.flush().unwrap();
            
            let mmap_file = MemoryMappedFile::open(temp_file.path()).unwrap();
            
            prop_assert_eq!(mmap_file.len(), data.len());
            prop_assert_eq!(mmap_file.file_size(), data.len() as u64);
            prop_assert_eq!(mmap_file.as_slice(), &data);
        }

        /// Fuzz test buffer operations with random data
        #[test]
        fn fuzz_adaptive_buffer_operations(
            device_type in device_type_strategy(),
            operations in prop::collection::vec(
                prop_oneof![
                    (0usize..=1024*1024).prop_map(|size| ("reserve", size)),
                    Just(("clear", 0)),
                    (1usize..=1024).prop_map(|size| ("split", size)),
                ], 
                0..=50
            )
        ) {
            let mut buffer = AdaptiveBuffer::new(device_type);
            
            for (op, size) in operations {
                match op {
                    "reserve" => {
                        let old_capacity = buffer.capacity();
                        buffer.reserve(size);
                        prop_assert!(buffer.capacity() >= old_capacity);
                    },
                    "clear" => {
                        buffer.clear();
                        prop_assert!(buffer.is_empty());
                    },
                    "split" => {
                        if buffer.len() >= size {
                            let split_data = buffer.split_to(size);
                            prop_assert_eq!(split_data.len(), size);
                        }
                    },
                    _ => {}
                }
                
                // Invariants that should always hold
                prop_assert!(buffer.len() <= buffer.capacity());
            }
        }
    }
}
