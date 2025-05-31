//! Function-level benchmarks for FerroCP
//!
//! This module provides detailed benchmarks for individual functions and methods
//! to identify performance bottlenecks at a granular level.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;

use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_io::{
    AdaptiveBuffer, AsyncFileReader, AsyncFileWriter, BufferedCopyEngine, CopyEngine,
    MicroFileCopyEngine, ParallelCopyEngine, PreReadBuffer, PreReadStrategy,
};
use ferrocp_types::DeviceType;

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = "A".repeat(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

/// Benchmark individual buffer operations
fn benchmark_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    let buffer_sizes = vec![
        (1024, "1KB"),
        (4096, "4KB"),
        (16384, "16KB"),
        (65536, "64KB"),
        (262144, "256KB"),
    ];

    for (size, size_name) in &buffer_sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark AdaptiveBuffer creation
        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer_create", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let buffer = AdaptiveBuffer::with_size(DeviceType::SSD, size);
                    black_box(buffer);
                });
            },
        );

        // Benchmark AdaptiveBuffer recreation (simulating resize)
        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer_recreate", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let _old_buffer = AdaptiveBuffer::with_size(DeviceType::SSD, 1024);
                    let new_buffer = AdaptiveBuffer::with_size(DeviceType::SSD, size);
                    black_box(new_buffer);
                });
            },
        );

        // Benchmark buffer clear operation
        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer_clear", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut buffer = AdaptiveBuffer::with_size(DeviceType::SSD, size);
                    // Fill buffer with data
                    let data = vec![0u8; size];
                    buffer.as_mut().copy_from_slice(&data);
                    // Clear buffer
                    buffer.clear();
                    black_box(buffer);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file I/O operations
fn benchmark_file_io_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_io_operations");

    let file_sizes = vec![
        (1024, "1KB"),
        (16384, "16KB"),
        (262144, "256KB"),
        (1048576, "1MB"),
    ];

    for (size, size_name) in &file_sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark AsyncFileReader open
        group.bench_with_input(
            BenchmarkId::new("async_reader_open", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);

                    rt.block_on(async {
                        let reader = AsyncFileReader::open(&source).await.unwrap();
                        black_box(reader);
                    });
                });
            },
        );

        // Benchmark AsyncFileWriter create
        group.bench_with_input(
            BenchmarkId::new("async_writer_create", size_name),
            size,
            |b, &_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let writer = AsyncFileWriter::create(&dest).await.unwrap();
                        black_box(writer);
                    });
                });
            },
        );

        // Benchmark read operation
        group.bench_with_input(
            BenchmarkId::new("async_reader_read", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);

                    rt.block_on(async {
                        let mut reader = AsyncFileReader::open(&source).await.unwrap();
                        let mut buffer = AdaptiveBuffer::with_size(DeviceType::SSD, size);
                        let bytes_read = reader.read_into_buffer(&mut buffer).await.unwrap();
                        black_box(bytes_read);
                    });
                });
            },
        );

        // Benchmark write operation
        group.bench_with_input(
            BenchmarkId::new("async_writer_write", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let dest = temp_dir.path().join("dest.txt");
                    let data = vec![0u8; size];

                    rt.block_on(async {
                        let mut writer = AsyncFileWriter::create(&dest).await.unwrap();
                        let bytes_written = writer.write_all(&data).await.unwrap();
                        black_box(bytes_written);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pre-read operations
fn benchmark_preread_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("preread_operations");

    let strategies = vec![
        (PreReadStrategy::SSD { size: 1024 * 1024 }, "SSD_1MB"),
        (PreReadStrategy::HDD { size: 64 * 1024 }, "HDD_64KB"),
        (PreReadStrategy::Network { size: 8 * 1024 }, "Network_8KB"),
    ];

    for (strategy, strategy_name) in &strategies {
        // Benchmark PreReadBuffer creation
        group.bench_with_input(
            BenchmarkId::new("preread_buffer_create", strategy_name),
            strategy,
            |b, strategy| {
                b.iter(|| {
                    let buffer = PreReadBuffer::with_strategy(DeviceType::SSD, *strategy);
                    black_box(buffer);
                });
            },
        );

        // Benchmark strategy creation with different sizes
        group.bench_with_input(
            BenchmarkId::new("preread_strategy_create_variants", strategy_name),
            strategy,
            |b, strategy| {
                b.iter(|| {
                    let buffer1 = PreReadBuffer::with_strategy(DeviceType::SSD, *strategy);
                    let buffer2 = PreReadBuffer::new(DeviceType::SSD);
                    black_box((buffer1, buffer2));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark engine initialization
fn benchmark_engine_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_initialization");

    // Benchmark MicroFileCopyEngine creation
    group.bench_function("micro_engine_new", |b| {
        b.iter(|| {
            let engine = MicroFileCopyEngine::new();
            black_box(engine);
        });
    });

    // Benchmark BufferedCopyEngine creation
    group.bench_function("buffered_engine_new", |b| {
        b.iter(|| {
            let engine = BufferedCopyEngine::new();
            black_box(engine);
        });
    });

    // Benchmark ParallelCopyEngine creation
    group.bench_function("parallel_engine_new", |b| {
        b.iter(|| {
            let engine = ParallelCopyEngine::new();
            black_box(engine);
        });
    });

    // Benchmark MicroFileCopyEngine with different strategies
    let strategies = vec![
        (ferrocp_io::MicroCopyStrategy::UltraFast, "UltraFast"),
        (ferrocp_io::MicroCopyStrategy::StackBuffer, "StackBuffer"),
        (ferrocp_io::MicroCopyStrategy::HyperFast, "HyperFast"),
        (ferrocp_io::MicroCopyStrategy::SuperFast, "SuperFast"),
    ];

    for (strategy, strategy_name) in &strategies {
        group.bench_with_input(
            BenchmarkId::new("micro_engine_with_strategy", strategy_name),
            strategy,
            |b, strategy| {
                b.iter(|| {
                    let engine = MicroFileCopyEngine::with_strategy(*strategy);
                    black_box(engine);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark device type detection
fn benchmark_device_detection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("device_detection");

    // Benchmark device detection for different path types
    let paths = vec![
        ("C:\\temp\\test.txt", "local_path"),
        ("\\\\server\\share\\test.txt", "network_path"),
        ("/tmp/test.txt", "unix_path"),
        ("./test.txt", "relative_path"),
    ];

    for (path, path_type) in &paths {
        group.bench_with_input(
            BenchmarkId::new("device_detect", path_type),
            path,
            |b, path| {
                b.iter(|| {
                    rt.block_on(async {
                        let engine = BufferedCopyEngine::new();
                        let device_type = engine
                            .detect_device_type(path)
                            .await
                            .unwrap_or(DeviceType::Unknown);
                        black_box(device_type);
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_buffer_operations,
    benchmark_file_io_operations,
    benchmark_preread_operations,
    benchmark_engine_initialization,
    benchmark_device_detection
);

criterion_main!(benches);
