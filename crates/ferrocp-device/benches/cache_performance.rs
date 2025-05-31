//! Performance benchmarks for device detection cache

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferrocp_device::{DeviceDetector, cache::DeviceCacheConfig};
use std::time::Duration;
use tempfile::TempDir;

fn bench_cache_hit_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("cache_hit_performance", |b| {
        b.to_async(&rt).iter(|| async {
            let detector = DeviceDetector::new();
            let temp_dir = TempDir::new().unwrap();
            let test_path = temp_dir.path().join("test_file.txt");
            std::fs::write(&test_path, "test content").unwrap();

            // First call to populate cache
            detector.detect_device_type_cached(&test_path).await.unwrap();

            // Benchmark subsequent cache hits
            for _ in 0..100 {
                black_box(detector.detect_device_type_cached(&test_path).await.unwrap());
            }
        });
    });
}

fn bench_cache_miss_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("cache_miss_performance", |b| {
        b.to_async(&rt).iter(|| async {
            let detector = DeviceDetector::new();
            let temp_dir = TempDir::new().unwrap();

            // Benchmark cache misses (different files each time)
            for i in 0..10 {
                let test_path = temp_dir.path().join(format!("test_file_{}.txt", i));
                std::fs::write(&test_path, "test content").unwrap();
                black_box(detector.detect_device_type_cached(&test_path).await.unwrap());
            }
        });
    });
}

fn bench_path_prefix_optimization(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("path_prefix_optimization", |b| {
        b.to_async(&rt).iter(|| async {
            let detector = DeviceDetector::new();
            let temp_dir = TempDir::new().unwrap();

            // Create multiple files in the same directory
            let files: Vec<_> = (0..50)
                .map(|i| {
                    let path = temp_dir.path().join(format!("file_{}.txt", i));
                    std::fs::write(&path, "test content").unwrap();
                    path
                })
                .collect();

            // First file should be a cache miss
            black_box(detector.detect_device_type_cached(&files[0]).await.unwrap());

            // Subsequent files in the same directory should be cache hits due to path prefix optimization
            for file in &files[1..] {
                black_box(detector.detect_device_type_cached(file).await.unwrap());
            }
        });
    });
}

fn bench_background_refresh(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("background_refresh", |b| {
        b.to_async(&rt).iter(|| async {
            let config = DeviceCacheConfig {
                enable_background_refresh: true,
                background_refresh_interval: Duration::from_millis(1),
                refresh_threshold: 0.1,
                ttl: Duration::from_millis(100),
                ..DeviceCacheConfig::default()
            };
            
            let detector = DeviceDetector::with_cache_config(config);
            let temp_dir = TempDir::new().unwrap();
            let test_path = temp_dir.path().join("test_file.txt");
            std::fs::write(&test_path, "test content").unwrap();

            // Initial detection
            detector.detect_device_type_cached(&test_path).await.unwrap();

            // Wait for refresh threshold
            tokio::time::sleep(Duration::from_millis(20)).await;

            // Trigger refresh queue population
            detector.detect_device_type_cached(&test_path).await.unwrap();

            // Process background refresh
            black_box(detector.process_background_refresh().await.unwrap());
        });
    });
}

fn bench_cache_statistics(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("cache_statistics", |b| {
        b.to_async(&rt).iter(|| async {
            let detector = DeviceDetector::new();
            let temp_dir = TempDir::new().unwrap();
            let test_path = temp_dir.path().join("test_file.txt");
            std::fs::write(&test_path, "test content").unwrap();

            // Perform some operations
            detector.detect_device_type_cached(&test_path).await.unwrap();
            detector.detect_device_type_cached(&test_path).await.unwrap();

            // Benchmark statistics collection
            black_box(detector.cache_stats().await);
        });
    });
}

criterion_group!(
    benches,
    bench_cache_hit_performance,
    bench_cache_miss_performance,
    bench_path_prefix_optimization,
    bench_background_refresh,
    bench_cache_statistics
);
criterion_main!(benches);
