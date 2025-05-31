//! Performance benchmarks for FerroCP
//!
//! These benchmarks measure the performance of core operations
//! and compare against baseline implementations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::fs;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_types::CompressionEngine;
use ferrocp_compression::CompressionEngineImpl;
use ferrocp_io::{BufferedCopyEngine, CopyEngine};

/// Helper function to create test data with patterns for compression testing
fn create_compressible_data(size: usize) -> Vec<u8> {
    let pattern = b"Hello, World! This is a test pattern for compression. ";
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

/// Helper function to create random test data
fn create_random_data(size: usize) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = Vec::with_capacity(size);
    let mut hasher = DefaultHasher::new();

    for i in 0..size {
        i.hash(&mut hasher);
        data.push((hasher.finish() % 256) as u8);
    }

    data
}

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = create_random_data(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

fn benchmark_compression(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("compression");

    // Test different data sizes
    let sizes = vec![
        64 * 1024,      // 64KB
        1024 * 1024,    // 1MB
    ];

    for size in &sizes {
        let test_data = create_compressible_data(*size);

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("compress", size),
            size,
            |b, &_size| {
                let data = test_data.clone();
                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let compression_engine = CompressionEngineImpl::new();
                        let result = compression_engine.compress(&data).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("decompress", size),
            size,
            |b, &_size| {
                let data = test_data.clone();
                let rt_handle = rt.handle().clone();
                let compressed_data = rt_handle.block_on(async {
                    let compression_engine = CompressionEngineImpl::new();
                    compression_engine.compress(&data).await.unwrap()
                });

                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let compression_engine = CompressionEngineImpl::new();
                        let result = compression_engine.decompress(&compressed_data).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file copy operations with optimized thresholds
fn benchmark_file_copy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_copy");

    // Test different file sizes including new threshold boundaries
    let sizes = vec![
        (1024, "1KB"),
        (4096, "4KB"),      // New micro file threshold
        (8192, "8KB"),      // Mid-range small file
        (16384, "16KB"),    // New small file threshold
        (64 * 1024, "64KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
    ];

    for (size, size_name) in &sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark BufferedCopyEngine
        group.bench_with_input(
            BenchmarkId::new("buffered_copy", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", size);
                    let dest = temp_dir.path().join("dest.dat");

                    rt.block_on(async {
                        let mut copy_engine = BufferedCopyEngine::new();
                        let result = copy_engine.copy_file(&source, &dest).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Compare with std::fs::copy
        group.bench_with_input(
            BenchmarkId::new("std_fs_copy", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", size);
                    let dest = temp_dir.path().join("dest.dat");

                    black_box(fs::copy(&source, &dest).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark compression algorithms
fn benchmark_compression_algorithms(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("compression_algorithms");

    let test_data = create_compressible_data(1024 * 1024); // 1MB
    group.throughput(Throughput::Bytes(test_data.len() as u64));

    // Benchmark different compression algorithms
    let algorithms = vec!["zstd", "lz4", "brotli"];

    for algorithm in &algorithms {
        group.bench_with_input(
            BenchmarkId::new("compress", algorithm),
            algorithm,
            |b, &_algorithm| {
                let data = test_data.clone();
                b.iter(|| {
                    rt.block_on(async {
                        let compression_engine = CompressionEngineImpl::new();
                        let result = compression_engine.compress(&data).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");

    // Test different buffer sizes
    let buffer_sizes = vec![
        (4 * 1024, "4KB"),
        (64 * 1024, "64KB"),
        (1024 * 1024, "1MB"),
    ];

    let file_size = 10 * 1024 * 1024; // 10MB file
    group.throughput(Throughput::Bytes(file_size as u64));

    for (buffer_size, buffer_name) in &buffer_sizes {
        group.bench_with_input(
            BenchmarkId::new("buffered_copy", buffer_name),
            buffer_size,
            |b, &_buffer_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");

                    rt.block_on(async {
                        let mut copy_engine = BufferedCopyEngine::new();
                        let result = copy_engine.copy_file(&source, &dest).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark threshold optimization effectiveness
fn benchmark_threshold_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("threshold_optimization");

    // Test files at the new threshold boundaries to verify optimization
    let test_cases = vec![
        (3072, "3KB_micro"),    // Should use MicroFileCopyEngine (< 4KB)
        (5120, "5KB_small"),    // Should use sync BufferedCopyEngine (< 16KB)
        (12288, "12KB_small"),  // Should use sync BufferedCopyEngine (< 16KB)
        (20480, "20KB_large"),  // Should use async BufferedCopyEngine (> 16KB)
    ];

    for (size, size_name) in &test_cases {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark with optimized thresholds
        group.bench_with_input(
            BenchmarkId::new("optimized_thresholds", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", size);
                    let dest = temp_dir.path().join("dest.dat");

                    rt.block_on(async {
                        let mut copy_engine = BufferedCopyEngine::new();
                        let result = copy_engine.copy_file(&source, &dest).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Compare with std::fs::copy for reference
        group.bench_with_input(
            BenchmarkId::new("std_fs_baseline", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", size);
                    let dest = temp_dir.path().join("dest.dat");

                    black_box(fs::copy(&source, &dest).unwrap())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_compression,
    benchmark_file_copy,
    benchmark_compression_algorithms,
    benchmark_memory_usage,
    benchmark_threshold_optimization
);

criterion_main!(benches);
