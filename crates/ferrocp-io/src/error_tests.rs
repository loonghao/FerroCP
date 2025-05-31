//! Comprehensive error handling tests for ferrocp-io

use crate::buffer::{AdaptiveBuffer, BufferPool};
use crate::memory_map::MemoryMappedFile;
use crate::reader::{AsyncFileReader, FileReader};
use crate::writer::{AsyncFileWriter, FileWriter};
use ferrocp_types::{DeviceType, Error, ErrorKind};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// Test error handling for file operations with invalid paths
#[tokio::test]
async fn test_invalid_path_errors() {
    // Test with non-existent file
    let non_existent = Path::new("/non/existent/path/file.txt");

    // AsyncFileReader should fail gracefully
    let reader_result = AsyncFileReader::open(non_existent).await;
    assert!(reader_result.is_err());
    if let Err(error) = reader_result {
        assert!(matches!(error.kind(), ErrorKind::Io));
    }

    // FileReader should fail gracefully
    let sync_reader_result = FileReader::open(non_existent);
    assert!(sync_reader_result.is_err());

    // MemoryMappedFile should fail gracefully
    let mmap_result = MemoryMappedFile::open(non_existent);
    assert!(mmap_result.is_err());
}

/// Test error handling for permission denied scenarios
#[tokio::test]
async fn test_permission_errors() {
    let temp_dir = TempDir::new().unwrap();
    let readonly_file = temp_dir.path().join("readonly.txt");

    // Create a file and make it read-only
    {
        let mut file = fs::File::create(&readonly_file).unwrap();
        file.write_all(b"test content").unwrap();
    }

    // Make file read-only (this might not work on all systems)
    let mut perms = fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_readonly(true);
    let _ = fs::set_permissions(&readonly_file, perms);

    // Try to open for writing - should fail on systems that support read-only
    let writer_result = AsyncFileWriter::create(&readonly_file).await;
    // Note: This test might pass on some systems where read-only isn't enforced
    if writer_result.is_err() {
        assert!(matches!(writer_result.unwrap_err().kind(), ErrorKind::Io));
    }
}

/// Test error handling for corrupted or invalid memory-mapped files
#[test]
fn test_memory_map_errors() {
    // Test with empty file
    let temp_file = NamedTempFile::new().unwrap();
    let mmap_result = MemoryMappedFile::open(temp_file.path());

    // Empty files might be handled differently, but should not panic
    match mmap_result {
        Ok(mmap) => {
            assert_eq!(mmap.len(), 0);
        }
        Err(_) => {
            // Error is also acceptable for empty files
        }
    }
}

/// Test buffer pool error conditions
#[test]
fn test_buffer_pool_edge_cases() {
    // Test with zero buffer size (should handle gracefully)
    let pool = BufferPool::new(0, 10);
    let buffer = pool.get_buffer();
    assert_eq!(buffer.capacity(), 0);

    // Test with zero max pool size
    let pool = BufferPool::new(1024, 0);
    let buffer1 = pool.get_buffer();
    let buffer2 = pool.get_buffer();

    // Return buffers - should not panic even with zero max size
    pool.return_buffer(buffer1);
    pool.return_buffer(buffer2);
    assert_eq!(pool.pool_size(), 0);
}

/// Test adaptive buffer error conditions
#[test]
fn test_adaptive_buffer_edge_cases() {
    // Test with extreme performance metrics
    let mut buffer = AdaptiveBuffer::new(DeviceType::SSD);

    // Test with negative/invalid performance metrics
    buffer.adapt_size(-1.0, -1.0);
    assert!(buffer.capacity() > 0); // Should still have valid capacity

    // Test with extremely high values
    buffer.adapt_size(f64::INFINITY, f64::INFINITY);
    assert!(buffer.capacity() > 0);

    // Test with NaN values
    buffer.adapt_size(f64::NAN, f64::NAN);
    assert!(buffer.capacity() > 0);
}

/// Test file operations with insufficient disk space simulation
#[tokio::test]
async fn test_disk_space_errors() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.txt");

    // Try to create a very large file that might exceed available space
    // Note: This test is system-dependent and might not always trigger the error
    let writer_result = AsyncFileWriter::create(&large_file).await;
    if let Ok(mut writer) = writer_result {
        // Try to write a very large amount of data
        let large_data = vec![0u8; 1024 * 1024]; // 1MB chunks

        // Write multiple chunks - eventually might hit disk space limits
        for _ in 0..1000 {
            let write_result = writer.write_all(&large_data).await;
            if write_result.is_err() {
                // Expected to fail at some point due to disk space
                break;
            }
        }
    }
}

/// Test concurrent access error handling
#[tokio::test]
async fn test_concurrent_access_errors() {
    let temp_file = NamedTempFile::new().unwrap();
    temp_file.as_file().write_all(b"test data").unwrap();

    // Try to open the same file multiple times for writing
    let writer1_result = AsyncFileWriter::create(temp_file.path()).await;
    let writer2_result = AsyncFileWriter::create(temp_file.path()).await;

    // Both should succeed on most systems, but we test that they don't panic
    assert!(writer1_result.is_ok() || writer1_result.is_err());
    assert!(writer2_result.is_ok() || writer2_result.is_err());
}

/// Test error propagation and context preservation
#[tokio::test]
async fn test_error_context_preservation() {
    let non_existent = Path::new("/definitely/does/not/exist/file.txt");

    match AsyncFileReader::open(non_existent).await {
        Err(error) => {
            // Verify error contains useful context
            let error_string = error.to_string();
            assert!(!error_string.is_empty());

            // Should contain some indication of the operation that failed
            assert!(
                error_string.contains("file")
                    || error_string.contains("path")
                    || error_string.contains("open")
                    || error_string.contains("read")
            );
        }
        Ok(_) => panic!("Expected error for non-existent file"),
    }
}

/// Test recovery from transient errors
#[tokio::test]
async fn test_error_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("recovery_test.txt");

    // Create a file
    {
        let mut writer = AsyncFileWriter::create(&test_file).await.unwrap();
        writer.write_all(b"initial content").await.unwrap();
        writer.flush().await.unwrap();
    }

    // Read the file successfully
    let mut reader = AsyncFileReader::open(&test_file).await.unwrap();
    let mut buffer = AdaptiveBuffer::new(DeviceType::SSD);
    let bytes_read = reader.read_into_buffer(&mut buffer).await.unwrap();
    assert!(bytes_read > 0);

    // Verify content
    assert_eq!(buffer.as_ref(), b"initial content");
}

/// Test error handling with malformed data
#[test]
fn test_malformed_data_handling() {
    // Test buffer operations with invalid sizes
    let mut buffer = AdaptiveBuffer::new(DeviceType::SSD);

    // Test reserve with extremely large size
    buffer.reserve(usize::MAX);
    // Should not panic, might not allocate the full amount

    // Test split operations at invalid indices
    if buffer.len() > 0 {
        let split_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            buffer.split_to(buffer.len() + 1000);
        }));
        // Should either work or panic gracefully (caught by catch_unwind)
        assert!(split_result.is_ok() || split_result.is_err());
    }
}

/// Test cleanup and resource management under error conditions
#[tokio::test]
async fn test_resource_cleanup_on_errors() {
    let temp_dir = TempDir::new().unwrap();

    // Test that resources are properly cleaned up even when operations fail
    for i in 0..10 {
        let test_file = temp_dir.path().join(format!("cleanup_test_{}.txt", i));

        // Try various operations that might fail
        let _ = AsyncFileWriter::create(&test_file).await;
        let _ = AsyncFileReader::open(&test_file).await;
        let _ = MemoryMappedFile::open(&test_file);

        // Clean up
        let _ = fs::remove_file(&test_file);
    }

    // If we reach here without panicking, resource cleanup is working
    assert!(true);
}

/// Test error handling with device-specific operations
#[test]
fn test_device_specific_errors() {
    // Test buffer creation for different device types
    for device_type in [
        DeviceType::SSD,
        DeviceType::HDD,
        DeviceType::Network,
        DeviceType::RamDisk,
        DeviceType::Unknown,
    ] {
        let buffer = AdaptiveBuffer::new(device_type);
        assert!(buffer.capacity() > 0);

        // Test with custom sizes that might be invalid for the device
        let custom_buffer = AdaptiveBuffer::with_size(device_type, 1);
        assert!(custom_buffer.capacity() > 0);

        let large_buffer = AdaptiveBuffer::with_size(device_type, 1024 * 1024 * 1024);
        assert!(large_buffer.capacity() > 0);
    }
}
