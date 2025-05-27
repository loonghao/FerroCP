//! Tests for async file operations core module

use py_eacopy::core::{EACopy, FileOperations, CopyStats};
use py_eacopy::config::Config;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Create a test file with specified content
async fn create_test_file(path: &PathBuf, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(path).await?;
    file.write_all(content).await?;
    file.flush().await?;
    Ok(())
}

/// Test basic file copying functionality
#[tokio::test]
async fn test_copy_file_basic() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.txt");
    let dest_path = temp_dir.path().join("dest.txt");
    
    // Create source file
    let test_content = b"Hello, World! This is a test file.";
    create_test_file(&source_path, test_content).await?;
    
    // Copy file
    let eacopy = EACopy::new();
    let stats = eacopy.copy_file(&source_path, &dest_path).await?;
    
    // Verify results
    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.bytes_copied, test_content.len() as u64);
    assert_eq!(stats.errors, 0);
    assert_eq!(stats.total_files_processed, 1);
    
    // Verify file content
    let copied_content = fs::read(&dest_path).await?;
    assert_eq!(copied_content, test_content);
    
    Ok(())
}

/// Test file copying with progress callback
#[tokio::test]
async fn test_copy_file_with_progress() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("large_source.txt");
    let dest_path = temp_dir.path().join("large_dest.txt");
    
    // Create a larger test file
    let test_content = vec![b'A'; 1024 * 1024]; // 1MB file
    create_test_file(&source_path, &test_content).await?;
    
    // Track progress updates
    let progress_updates = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_updates_clone = progress_updates.clone();
    
    let eacopy = EACopy::new().with_progress_callback(move |progress| {
        let mut updates = progress_updates_clone.lock().unwrap();
        updates.push((progress.current_bytes, progress.current_total));
    });
    
    let stats = eacopy.copy_file(&source_path, &dest_path).await?;
    
    // Verify results
    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.bytes_copied, test_content.len() as u64);
    assert!(stats.avg_speed > 0.0);
    
    // Verify progress was reported
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty());
    
    // Verify final progress shows completion
    if let Some((current, total)) = updates.last() {
        assert_eq!(*current, *total);
        assert_eq!(*total, test_content.len() as u64);
    }
    
    Ok(())
}

/// Test directory copying functionality
#[tokio::test]
async fn test_copy_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source_dir");
    let dest_dir = temp_dir.path().join("dest_dir");
    
    // Create source directory structure
    fs::create_dir_all(&source_dir).await?;
    fs::create_dir_all(source_dir.join("subdir")).await?;
    
    // Create test files
    create_test_file(&source_dir.join("file1.txt"), b"Content 1").await?;
    create_test_file(&source_dir.join("file2.txt"), b"Content 2").await?;
    create_test_file(&source_dir.join("subdir").join("file3.txt"), b"Content 3").await?;
    
    // Copy directory
    let eacopy = EACopy::new();
    let stats = eacopy.copy_directory(&source_dir, &dest_dir).await?;
    
    // Verify results
    assert_eq!(stats.files_copied, 3);
    assert!(stats.directories_created >= 1);
    assert_eq!(stats.errors, 0);
    
    // Verify files exist and have correct content
    assert!(dest_dir.join("file1.txt").exists());
    assert!(dest_dir.join("file2.txt").exists());
    assert!(dest_dir.join("subdir").join("file3.txt").exists());
    
    let content1 = fs::read(dest_dir.join("file1.txt")).await?;
    assert_eq!(content1, b"Content 1");
    
    Ok(())
}

/// Test batch file copying
#[tokio::test]
async fn test_copy_files_batch() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    
    // Create multiple source files
    let mut file_pairs = Vec::new();
    for i in 0..5 {
        let source_path = temp_dir.path().join(format!("source_{}.txt", i));
        let dest_path = temp_dir.path().join(format!("dest_{}.txt", i));
        
        let content = format!("Content for file {}", i);
        create_test_file(&source_path, content.as_bytes()).await?;
        
        file_pairs.push((source_path, dest_path));
    }
    
    // Copy files in batch
    let config = Config::new().with_thread_count(3);
    let eacopy = EACopy::with_config(config);
    let stats = eacopy.copy_files_batch(file_pairs.clone()).await?;
    
    // Verify results
    assert_eq!(stats.files_copied, 5);
    assert_eq!(stats.errors, 0);
    assert_eq!(stats.concurrent_operations, 3);
    
    // Verify all files were copied
    for (_, dest_path) in file_pairs {
        assert!(dest_path.exists());
    }
    
    Ok(())
}

/// Test error handling for non-existent source
#[tokio::test]
async fn test_copy_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("nonexistent.txt");
    let dest_path = temp_dir.path().join("dest.txt");
    
    let eacopy = EACopy::new();
    let result = eacopy.copy_file(&source_path, &dest_path).await;
    
    assert!(result.is_err());
}

/// Test copy statistics accuracy
#[tokio::test]
async fn test_copy_stats_accuracy() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("test_file.txt");
    let dest_path = temp_dir.path().join("copied_file.txt");
    
    // Create test file with known size
    let test_content = vec![b'X'; 12345]; // Specific size for testing
    create_test_file(&source_path, &test_content).await?;
    
    let eacopy = EACopy::new();
    let stats = eacopy.copy_file(&source_path, &dest_path).await?;
    
    // Verify statistics
    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.bytes_copied, 12345);
    assert_eq!(stats.files_skipped, 0);
    assert_eq!(stats.errors, 0);
    assert_eq!(stats.total_files_processed, 1);
    assert!(stats.duration.as_millis() > 0);
    assert!(stats.avg_speed > 0.0);
    assert_eq!(stats.success_rate(), 100.0);
    
    Ok(())
}

/// Test configuration impact on performance
#[tokio::test]
async fn test_config_buffer_size() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("large_file.txt");
    let dest_path1 = temp_dir.path().join("dest1.txt");
    let dest_path2 = temp_dir.path().join("dest2.txt");
    
    // Create a large test file
    let test_content = vec![b'B'; 1024 * 1024]; // 1MB
    create_test_file(&source_path, &test_content).await?;
    
    // Test with small buffer
    let config1 = Config::new().with_buffer_size(4096); // 4KB
    let eacopy1 = EACopy::with_config(config1);
    let stats1 = eacopy1.copy_file(&source_path, &dest_path1).await?;
    
    // Test with large buffer
    let config2 = Config::new().with_buffer_size(256 * 1024); // 256KB
    let eacopy2 = EACopy::with_config(config2);
    let stats2 = eacopy2.copy_file(&source_path, &dest_path2).await?;
    
    // Both should copy the same amount of data
    assert_eq!(stats1.bytes_copied, stats2.bytes_copied);
    assert_eq!(stats1.files_copied, stats2.files_copied);
    
    // Verify files are identical
    let content1 = fs::read(&dest_path1).await?;
    let content2 = fs::read(&dest_path2).await?;
    assert_eq!(content1, content2);
    assert_eq!(content1, test_content);
    
    Ok(())
}
