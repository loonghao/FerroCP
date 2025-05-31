//! Integration tests for FerroCP
//!
//! These tests verify that all components work together correctly
//! in real-world scenarios.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

use ferrocp_types::CompressionEngine;
use ferrocp_io::{BufferedCopyEngine, CopyEngine};
use ferrocp_compression::CompressionEngineImpl;

/// Helper function to create test files with specific content
fn create_test_file(path: &Path, size: usize) -> std::io::Result<()> {
    let content = "A".repeat(size);
    fs::write(path, content)
}

/// Helper function to create test files with random content
fn create_random_test_file(path: &Path, size: usize) -> std::io::Result<()> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut content = Vec::with_capacity(size);
    let mut hasher = DefaultHasher::new();

    for i in 0..size {
        i.hash(&mut hasher);
        content.push((hasher.finish() % 256) as u8);
    }

    fs::write(path, content)
}

/// Helper function to create directory structure with files
fn create_test_directory_structure(base_path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut created_files = Vec::new();

    // Create subdirectories
    let sub_dirs = ["subdir1", "subdir2", "subdir1/nested"];
    for dir in &sub_dirs {
        fs::create_dir_all(base_path.join(dir))?;
    }

    // Create files of different sizes
    let files = [
        ("small.txt", 1024),           // 1KB
        ("medium.txt", 64 * 1024),     // 64KB
        ("large.txt", 1024 * 1024),    // 1MB
        ("subdir1/file1.txt", 2048),
        ("subdir2/file2.txt", 4096),
        ("subdir1/nested/file3.txt", 8192),
    ];

    for (file_path, size) in &files {
        let full_path = base_path.join(file_path);
        create_test_file(&full_path, *size)?;
        created_files.push(full_path);
    }

    Ok(created_files)
}

#[tokio::test]
async fn test_basic_file_copy() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_file = temp_dir.path().join("source.txt");
    let dest_file = temp_dir.path().join("dest.txt");

    // Create a test file
    create_test_file(&source_file, 1024)?; // 1KB file

    // Initialize copy engine
    let mut copy_engine = BufferedCopyEngine::new();

    // Perform copy operation
    let result = copy_engine.copy_file(&source_file, &dest_file).await?;

    // Verify the copy was successful
    assert!(dest_file.exists());
    assert_eq!(fs::metadata(&source_file)?.len(), fs::metadata(&dest_file)?.len());
    assert_eq!(fs::read(&source_file)?, fs::read(&dest_file)?);
    assert!(result.bytes_copied > 0);

    Ok(())
}

#[tokio::test]
async fn test_compression_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create test data
    let test_data = "Hello, World! ".repeat(1000); // Repeatable data for good compression
    let original_data = test_data.as_bytes();

    // Initialize compression engine
    let compression_engine = CompressionEngineImpl::new();

    // Compress the data
    let compressed_data = compression_engine.compress(original_data).await?;

    // Verify compression worked
    assert!(compressed_data.len() < original_data.len(), "Data should be compressed");

    // Decompress the data
    let decompressed_data = compression_engine.decompress(&compressed_data).await?;

    // Verify decompression worked
    assert_eq!(original_data, decompressed_data.as_slice());

    Ok(())
}

#[tokio::test]
async fn test_large_file_copy() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_file = temp_dir.path().join("large_source.dat");
    let dest_file = temp_dir.path().join("large_dest.dat");

    // Create a large test file (10MB)
    create_random_test_file(&source_file, 10 * 1024 * 1024)?;

    // Test with buffered copy engine for large files
    let mut copy_engine = BufferedCopyEngine::new();
    let result = copy_engine.copy_file(&source_file, &dest_file).await?;

    // Verify the copy was successful
    assert!(dest_file.exists());
    assert_eq!(fs::metadata(&source_file)?.len(), fs::metadata(&dest_file)?.len());
    assert_eq!(result.bytes_copied, 10 * 1024 * 1024);

    // Verify content integrity
    let source_content = fs::read(&source_file)?;
    let dest_content = fs::read(&dest_file)?;
    assert_eq!(source_content, dest_content);

    Ok(())
}

#[tokio::test]
async fn test_directory_copy_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_dir = temp_dir.path().join("source");
    let dest_dir = temp_dir.path().join("dest");

    // Create source directory structure
    fs::create_dir_all(&source_dir)?;
    let created_files = create_test_directory_structure(&source_dir)?;

    // Copy each file individually using BufferedCopyEngine
    for file_path in &created_files {
        let relative_path = file_path.strip_prefix(&source_dir)?;
        let dest_file = dest_dir.join(relative_path);

        // Ensure destination directory exists
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy the file
        let mut copy_engine = BufferedCopyEngine::new();
        let result = copy_engine.copy_file(file_path, &dest_file).await?;
        assert!(result.bytes_copied > 0);
    }

    // Verify all files were copied successfully
    for source_file in &created_files {
        let relative_path = source_file.strip_prefix(&source_dir)?;
        let dest_file = dest_dir.join(relative_path);

        assert!(dest_file.exists());
        assert_eq!(fs::metadata(source_file)?.len(), fs::metadata(&dest_file)?.len());

        // Verify content integrity
        let source_content = fs::read(source_file)?;
        let dest_content = fs::read(&dest_file)?;
        assert_eq!(source_content, dest_content);
    }

    Ok(())
}

#[tokio::test]
async fn test_multiple_copy_engines_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_file = temp_dir.path().join("source.dat");
    let dest_file1 = temp_dir.path().join("dest1.dat");
    let dest_file2 = temp_dir.path().join("dest2.dat");

    // Create a test file
    create_random_test_file(&source_file, 1024 * 1024)?; // 1MB

    // Test with BufferedCopyEngine
    let mut buffered_engine = BufferedCopyEngine::new();
    let result1 = buffered_engine.copy_file(&source_file, &dest_file1).await?;

    // Test with another BufferedCopyEngine instance (different configuration)
    let mut second_engine = BufferedCopyEngine::new();
    let result2 = second_engine.copy_file(&source_file, &dest_file2).await?;

    // Verify both copies were successful
    assert!(dest_file1.exists());
    assert!(dest_file2.exists());
    assert_eq!(result1.bytes_copied, result2.bytes_copied);
    assert_eq!(result1.bytes_copied, 1024 * 1024);

    // Verify content integrity for both copies
    let source_content = fs::read(&source_file)?;
    let dest_content1 = fs::read(&dest_file1)?;
    let dest_content2 = fs::read(&dest_file2)?;

    assert_eq!(source_content, dest_content1);
    assert_eq!(source_content, dest_content2);

    Ok(())
}

#[tokio::test]
async fn test_error_recovery_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_file = temp_dir.path().join("source.txt");

    // Create source file
    create_test_file(&source_file, 1024)?;

    // Test 1: Try to copy to a non-existent directory (should fail)
    let nonexistent_dest = temp_dir.path().join("nonexistent/deep/path/dest.txt");
    let mut copy_engine = BufferedCopyEngine::new();
    let result = copy_engine.copy_file(&source_file, &nonexistent_dest).await;

    // Verify error handling
    assert!(result.is_err(), "Copy to non-existent directory should fail");

    // Test 2: Create the directory and retry (should succeed)
    fs::create_dir_all(nonexistent_dest.parent().unwrap())?;
    let result = copy_engine.copy_file(&source_file, &nonexistent_dest).await?;
    assert!(result.bytes_copied > 0);
    assert!(nonexistent_dest.exists());

    // Test 3: Unix-specific permission test
    #[cfg(unix)]
    {
        let readonly_dest = temp_dir.path().join("readonly_dir/dest.txt");
        fs::create_dir_all(readonly_dest.parent().unwrap())?;

        // Make directory read-only
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(readonly_dest.parent().unwrap())?.permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(readonly_dest.parent().unwrap(), perms)?;

        // Attempt copy operation (should fail)
        let result = copy_engine.copy_file(&source_file, &readonly_dest).await;
        assert!(result.is_err(), "Copy to read-only directory should fail");

        // Restore permissions and retry
        let mut perms = fs::metadata(readonly_dest.parent().unwrap())?.permissions();
        perms.set_mode(0o755); // Read-write-execute
        fs::set_permissions(readonly_dest.parent().unwrap(), perms)?;

        // Retry copy operation (should succeed)
        let result = copy_engine.copy_file(&source_file, &readonly_dest).await?;
        assert!(result.bytes_copied > 0);
        assert!(readonly_dest.exists());
    }

    // Test 4: Windows-specific test (just verify basic functionality)
    #[cfg(windows)]
    {
        let windows_dest = temp_dir.path().join("windows_test/dest.txt");
        fs::create_dir_all(windows_dest.parent().unwrap())?;

        let result = copy_engine.copy_file(&source_file, &windows_dest).await?;
        assert!(result.bytes_copied > 0);
        assert!(windows_dest.exists());
    }

    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_file = temp_dir.path().join("source.txt");
    let dest_file = temp_dir.path().join("dest.txt");

    // Create a test file
    create_test_file(&source_file, 1024)?;

    // Test with very short timeout (should complete successfully for small file)
    let copy_operation = async {
        let mut copy_engine = BufferedCopyEngine::new();
        copy_engine.copy_file(&source_file, &dest_file).await
    };

    let result = timeout(Duration::from_secs(5), copy_operation).await?;
    assert!(result.is_ok());
    assert!(dest_file.exists());

    Ok(())
}
