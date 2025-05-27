//! Example demonstrating intelligent compression engine

use ferrocp::compression::{CompressionEngine, NetworkMonitor};
use ferrocp::config::CompressionConfig;
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Testing intelligent compression engine...");

    // Test 1: Basic compression functionality
    println!("\n1. Testing basic compression...");
    let config = CompressionConfig {
        enabled: true,
        level: 6,
        adaptive: false,
        min_file_size: 1024,
        dictionary_path: None,
    };

    let engine = CompressionEngine::new(config);
    
    // Create test data
    let test_data = "Hello, World! ".repeat(1000); // Repeating data compresses well
    let input = Cursor::new(test_data.as_bytes());
    let mut output = Vec::new();

    let result = engine.compress_stream(input, &mut output, test_data.len() as u64).await?;
    
    println!("Compression result:");
    println!("  Original size: {} bytes", result.original_size);
    println!("  Compressed size: {} bytes", result.compressed_size);
    println!("  Compression ratio: {:.3}", result.compression_ratio);
    println!("  Savings: {:.1}%", result.savings_percentage());
    println!("  Throughput: {:.2} MB/s", result.throughput_mbps());
    println!("  Duration: {:?}", result.duration);

    // Test 2: Decompression
    println!("\n2. Testing decompression...");
    let compressed_input = Cursor::new(&output);
    let mut decompressed_output = Vec::new();

    let decomp_result = engine.decompress_stream(compressed_input, &mut decompressed_output).await?;
    
    println!("Decompression result:");
    println!("  Decompressed size: {} bytes", decomp_result.compressed_size);
    println!("  Duration: {:?}", decomp_result.duration);
    
    // Verify data integrity
    let decompressed_string = String::from_utf8(decompressed_output)?;
    let original_string = test_data;
    
    if decompressed_string == original_string {
        println!("  ✓ Data integrity verified");
    } else {
        println!("  ✗ Data integrity check failed");
    }

    // Test 3: Network monitoring
    println!("\n3. Testing network monitoring...");
    let monitor = NetworkMonitor::new();
    
    // Simulate network measurements
    for i in 1..=10 {
        let bytes = 1024 * 1024; // 1MB
        let duration = Duration::from_millis(100 + i * 10); // Varying speed
        monitor.update_throughput(bytes, duration);
        
        println!("  Measurement {}: {:.2} MB/s", i, monitor.get_throughput() / (1024.0 * 1024.0));
        sleep(Duration::from_millis(50)).await;
    }
    
    println!("Network statistics:");
    println!("  Current throughput: {:.2} MB/s", monitor.get_throughput() / (1024.0 * 1024.0));
    println!("  Peak throughput: {:.2} MB/s", monitor.get_peak_throughput() / (1024.0 * 1024.0));
    println!("  Average throughput: {:.2} MB/s", monitor.get_avg_throughput() / (1024.0 * 1024.0));
    println!("  Smoothed throughput: {:.2} MB/s", monitor.get_smoothed_throughput() / (1024.0 * 1024.0));
    println!("  Network stable: {}", monitor.is_network_stable());

    // Test 4: Adaptive compression
    println!("\n4. Testing adaptive compression...");
    let adaptive_config = CompressionConfig {
        enabled: true,
        level: 6,
        adaptive: true,
        min_file_size: 1024,
        dictionary_path: None,
    };

    let adaptive_engine = CompressionEngine::new(adaptive_config);
    
    // Test different network speeds
    let network_speeds = vec![
        1.0 * 1024.0 * 1024.0,    // 1 MB/s - slow
        10.0 * 1024.0 * 1024.0,   // 10 MB/s - medium
        100.0 * 1024.0 * 1024.0,  // 100 MB/s - fast
        1000.0 * 1024.0 * 1024.0, // 1 GB/s - very fast
    ];

    for speed in network_speeds {
        let level = adaptive_engine.adaptive_compression_level(speed);
        println!("  Network speed: {:.0} MB/s -> Compression level: {}", 
                 speed / (1024.0 * 1024.0), level);
    }

    // Test 5: Compression statistics
    println!("\n5. Testing compression statistics...");
    
    // Perform multiple compressions
    for i in 1..=5 {
        let data = format!("Test data {} ", i).repeat(500);
        let input = Cursor::new(data.as_bytes());
        let mut output = Vec::new();
        
        let _result = adaptive_engine.compress_stream(input, &mut output, data.len() as u64).await?;
    }

    let stats = adaptive_engine.get_stats();
    println!("Engine statistics:");
    println!("  Total operations: {}", stats.total_operations);
    println!("  Compression operations: {}", stats.compression_operations);
    println!("  Total bytes processed: {}", stats.total_bytes_processed);
    println!("  Total bytes compressed: {}", stats.total_bytes_compressed);
    println!("  Overall compression ratio: {:.3}", stats.overall_compression_ratio);
    println!("  Best compression ratio: {:.3}", stats.best_compression_ratio);
    println!("  Worst compression ratio: {:.3}", stats.worst_compression_ratio);
    println!("  Average compression level: {:.1}", stats.avg_compression_level);
    println!("  Adaptive level changes: {}", stats.adaptive_level_changes);

    // Test 6: Compression benefit estimation
    println!("\n6. Testing compression benefit estimation...");
    
    let file_sizes = vec![512, 2048, 10240, 102400]; // Different file sizes
    let network_speed = 50.0 * 1024.0 * 1024.0; // 50 MB/s
    
    for size in file_sizes {
        let estimate = adaptive_engine.estimate_compression_benefit(size, network_speed);
        println!("  File size: {} bytes", size);
        println!("    Recommended: {}", estimate.recommended);
        println!("    Estimated ratio: {:.3}", estimate.estimated_ratio);
        println!("    Estimated time savings: {:?}", estimate.estimated_time_savings);
        println!("    Estimated bandwidth savings: {} bytes", estimate.estimated_bandwidth_savings);
    }

    // Test 7: Performance comparison
    println!("\n7. Testing performance with different compression levels...");
    
    let large_data = "Performance test data ".repeat(10000); // ~230KB
    let levels = vec![1, 3, 6, 9, 15];
    
    for level in levels {
        let config = CompressionConfig {
            enabled: true,
            level,
            adaptive: false,
            min_file_size: 1024,
            dictionary_path: None,
        };
        
        let engine = CompressionEngine::new(config);
        let input = Cursor::new(large_data.as_bytes());
        let mut output = Vec::new();
        
        let result = engine.compress_stream(input, &mut output, large_data.len() as u64).await?;
        
        println!("  Level {}: ratio={:.3}, throughput={:.2} MB/s, time={:?}",
                 level, result.compression_ratio, result.throughput_mbps(), result.duration);
    }

    println!("\n✅ All compression engine tests completed successfully!");
    Ok(())
}
