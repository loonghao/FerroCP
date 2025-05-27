//! Example demonstrating async file operations

use py_eacopy::core::{EACopy, FileOperations};
use py_eacopy::config::Config;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Testing async file operations...");

    // Create temporary directory
    let temp_dir = std::env::temp_dir().join("py_eacopy_test");
    fs::create_dir_all(&temp_dir).await?;

    // Test 1: Basic file copy
    println!("\n1. Testing basic file copy...");
    let source_file = temp_dir.join("source.txt");
    let dest_file = temp_dir.join("dest.txt");

    // Create source file
    let mut file = fs::File::create(&source_file).await?;
    file.write_all(b"Hello, World! This is a test file for async operations.").await?;
    file.flush().await?;
    drop(file);

    // Copy file
    let eacopy = EACopy::new();
    let stats = eacopy.copy_file(&source_file, &dest_file).await?;

    println!("Copy completed:");
    println!("  Files copied: {}", stats.files_copied);
    println!("  Bytes copied: {}", stats.bytes_copied);
    println!("  Duration: {:?}", stats.duration);
    println!("  Speed: {:.2} MB/s", stats.throughput_mbps());

    // Verify content
    let copied_content = fs::read(&dest_file).await?;
    let original_content = fs::read(&source_file).await?;
    assert_eq!(copied_content, original_content);
    println!("  ✓ Content verification passed");

    // Test 2: Copy with progress callback
    println!("\n2. Testing copy with progress callback...");
    let large_source = temp_dir.join("large_source.txt");
    let large_dest = temp_dir.join("large_dest.txt");

    // Create a larger file
    let large_content = vec![b'A'; 1024 * 100]; // 100KB
    fs::write(&large_source, &large_content).await?;

    let eacopy_with_progress = EACopy::new().with_progress_callback(|progress| {
        let percent = if progress.current_total > 0 {
            (progress.current_bytes as f64 / progress.current_total as f64) * 100.0
        } else {
            0.0
        };
        print!("\r  Progress: {:.1}% ({} / {} bytes)", 
               percent, progress.current_bytes, progress.current_total);
    });

    let stats = eacopy_with_progress.copy_file(&large_source, &large_dest).await?;
    println!("\n  Copy completed: {} bytes in {:?}", stats.bytes_copied, stats.duration);

    // Test 3: Directory copy
    println!("\n3. Testing directory copy...");
    let source_dir = temp_dir.join("source_dir");
    let dest_dir = temp_dir.join("dest_dir");

    // Create source directory structure
    fs::create_dir_all(&source_dir).await?;
    fs::create_dir_all(source_dir.join("subdir")).await?;

    // Create test files
    fs::write(source_dir.join("file1.txt"), b"Content 1").await?;
    fs::write(source_dir.join("file2.txt"), b"Content 2").await?;
    fs::write(source_dir.join("subdir").join("file3.txt"), b"Content 3").await?;

    let stats = eacopy.copy_directory(&source_dir, &dest_dir).await?;
    println!("Directory copy completed:");
    println!("  Files copied: {}", stats.files_copied);
    println!("  Directories created: {}", stats.directories_created);
    println!("  Total bytes: {}", stats.bytes_copied);

    // Test 4: Batch copy
    println!("\n4. Testing batch copy...");
    let mut file_pairs = Vec::new();
    for i in 0..5 {
        let src = temp_dir.join(format!("batch_src_{}.txt", i));
        let dst = temp_dir.join(format!("batch_dst_{}.txt", i));
        
        fs::write(&src, format!("Batch file content {}", i)).await?;
        file_pairs.push((src, dst));
    }

    let config = Config::new().with_thread_count(3);
    let batch_eacopy = EACopy::with_config(config);
    let stats = batch_eacopy.copy_files_batch(file_pairs).await?;
    
    println!("Batch copy completed:");
    println!("  Files copied: {}", stats.files_copied);
    println!("  Concurrent operations: {}", stats.concurrent_operations);
    println!("  Success rate: {:.1}%", stats.success_rate());

    // Test 5: Configuration impact
    println!("\n5. Testing configuration impact...");
    let test_file = temp_dir.join("config_test.txt");
    let dest1 = temp_dir.join("config_dest1.txt");
    let dest2 = temp_dir.join("config_dest2.txt");

    // Create test file
    let test_content = vec![b'X'; 1024 * 50]; // 50KB
    fs::write(&test_file, &test_content).await?;

    // Test with small buffer
    let config1 = Config::new().with_buffer_size(4096);
    let eacopy1 = EACopy::with_config(config1);
    let start1 = std::time::Instant::now();
    let stats1 = eacopy1.copy_file(&test_file, &dest1).await?;
    let duration1 = start1.elapsed();

    // Test with large buffer
    let config2 = Config::new().with_buffer_size(64 * 1024);
    let eacopy2 = EACopy::with_config(config2);
    let start2 = std::time::Instant::now();
    let stats2 = eacopy2.copy_file(&test_file, &dest2).await?;
    let duration2 = start2.elapsed();

    println!("Configuration comparison:");
    println!("  Small buffer (4KB): {:?}, {:.2} MB/s", duration1, stats1.throughput_mbps());
    println!("  Large buffer (64KB): {:?}, {:.2} MB/s", duration2, stats2.throughput_mbps());

    // Cleanup
    println!("\n6. Cleaning up...");
    fs::remove_dir_all(&temp_dir).await?;
    println!("  ✓ Cleanup completed");

    println!("\n✅ All async file operation tests passed!");
    Ok(())
}
