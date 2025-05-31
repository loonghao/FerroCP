//! Micro-benchmarks for individual function performance
//!
//! This benchmark suite provides detailed performance analysis at the function level,
//! testing different file sizes and operation types to identify optimization opportunities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrocp_io::{
    AsyncFileReader, AsyncFileWriter, BufferedCopyEngine, CopyEngine, CopyOptions,
    MicroCopyStrategy, MicroFileCopyEngine, ParallelCopyEngine, PreReadStrategy,
};
use std::fs;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Generate test data with realistic patterns
fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push(((i * 7 + 13) % 256) as u8);
    }
    data
}

/// Create a temporary file with test data
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = generate_test_data(size);
    fs::write(&file_path, data).expect("Failed to write test file");
    file_path
}

/// Benchmark file reading operations
fn bench_file_reading(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_reading");

    let test_sizes = vec![
        ("1B", 1),
        ("1KB", 1024),
        ("4KB", 4 * 1024),
        ("16KB", 16 * 1024),
        ("64KB", 64 * 1024),
        ("256KB", 256 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
    ];

    for (size_name, file_size) in test_sizes {
        group.throughput(Throughput::Bytes(file_size as u64));

        // Benchmark std::fs::read
        group.bench_with_input(
            BenchmarkId::new("std_fs_read", size_name),
            &file_size,
            |b, &file_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = create_test_file(&temp_dir, "test.dat", file_size);
                    black_box(fs::read(&file_path).unwrap())
                });
            },
        );

        // Benchmark AsyncFileReader
        group.bench_with_input(
            BenchmarkId::new("async_file_reader", size_name),
            &file_size,
            |b, &file_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = create_test_file(&temp_dir, "test.dat", file_size);
                    rt.block_on(async {
                        let mut reader = AsyncFileReader::open(&file_path).await.unwrap();
                        let mut buffer = vec![0u8; file_size];
                        black_box(reader.read_exact(&mut buffer).await.unwrap())
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file writing operations
fn bench_file_writing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_writing");

    let test_sizes = vec![
        ("1B", 1),
        ("1KB", 1024),
        ("4KB", 4 * 1024),
        ("16KB", 16 * 1024),
        ("64KB", 64 * 1024),
        ("256KB", 256 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
    ];

    for (size_name, file_size) in test_sizes {
        group.throughput(Throughput::Bytes(file_size as u64));
        let test_data = generate_test_data(file_size);

        // Benchmark std::fs::write
        group.bench_with_input(
            BenchmarkId::new("std_fs_write", size_name),
            &test_data,
            |b, test_data| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = temp_dir.path().join("test.dat");
                    black_box(fs::write(&file_path, test_data).unwrap())
                });
            },
        );

        // Benchmark AsyncFileWriter
        group.bench_with_input(
            BenchmarkId::new("async_file_writer", size_name),
            &test_data,
            |b, test_data| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = temp_dir.path().join("test.dat");
                    rt.block_on(async {
                        let mut writer = AsyncFileWriter::create(&file_path).await.unwrap();
                        black_box(writer.write_all(test_data).await.unwrap())
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark buffer operations
fn bench_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    let buffer_sizes = vec![
        ("4KB", 4 * 1024),
        ("16KB", 16 * 1024),
        ("64KB", 64 * 1024),
        ("256KB", 256 * 1024),
        ("1MB", 1024 * 1024),
    ];

    for (size_name, buffer_size) in buffer_sizes {
        group.throughput(Throughput::Bytes(buffer_size as u64));

        // Benchmark Vec allocation
        group.bench_with_input(
            BenchmarkId::new("vec_allocation", size_name),
            &buffer_size,
            |b, &buffer_size| {
                b.iter(|| black_box(vec![0u8; buffer_size]));
            },
        );

        // Benchmark Vec::with_capacity
        group.bench_with_input(
            BenchmarkId::new("vec_with_capacity", size_name),
            &buffer_size,
            |b, &buffer_size| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(buffer_size);
                    vec.resize(buffer_size, 0);
                    black_box(vec)
                });
            },
        );

        // Benchmark memory copy
        group.bench_with_input(
            BenchmarkId::new("memory_copy", size_name),
            &buffer_size,
            |b, &buffer_size| {
                let source = vec![0xAB; buffer_size];
                b.iter(|| {
                    let mut dest = vec![0u8; buffer_size];
                    dest.copy_from_slice(&source);
                    black_box(dest)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark copy engine selection and performance
fn bench_copy_engines(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("copy_engines");

    let test_sizes = vec![
        ("1KB", 1024),
        ("4KB", 4 * 1024),
        ("16KB", 16 * 1024),
        ("64KB", 64 * 1024),
        ("1MB", 1024 * 1024),
        ("10MB", 10 * 1024 * 1024),
    ];

    for (size_name, file_size) in test_sizes {
        group.throughput(Throughput::Bytes(file_size as u64));

        // Benchmark MicroFileCopyEngine
        if file_size <= 4 * 1024 {
            group.bench_with_input(
                BenchmarkId::new("micro_copy_engine", size_name),
                &file_size,
                |b, &file_size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.dat", file_size);
                        let dest = temp_dir.path().join("dest.dat");

                        rt.block_on(async {
                            let mut engine =
                                MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraFast);
                            black_box(engine.copy_file(&source, &dest).await.unwrap())
                        })
                    });
                },
            );
        }

        // Benchmark BufferedCopyEngine
        group.bench_with_input(
            BenchmarkId::new("buffered_copy_engine", size_name),
            &file_size,
            |b, &file_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        black_box(engine.copy_file(&source, &dest).await.unwrap())
                    })
                });
            },
        );

        // Benchmark ParallelCopyEngine for larger files
        if file_size >= 1024 * 1024 {
            group.bench_with_input(
                BenchmarkId::new("parallel_copy_engine", size_name),
                &file_size,
                |b, &file_size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.dat", file_size);
                        let dest = temp_dir.path().join("dest.dat");

                        rt.block_on(async {
                            let mut engine = ParallelCopyEngine::new();
                            black_box(engine.copy_file(&source, &dest).await.unwrap())
                        })
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark preread strategies
fn bench_preread_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("preread_strategies");

    let file_size = 50 * 1024 * 1024; // 50MB
    group.throughput(Throughput::Bytes(file_size as u64));

    let strategies = vec![
        ("no_preread", None),
        (
            "256KB_preread",
            Some(PreReadStrategy::SSD { size: 256 * 1024 }),
        ),
        (
            "512KB_preread",
            Some(PreReadStrategy::SSD { size: 512 * 1024 }),
        ),
        (
            "1MB_preread",
            Some(PreReadStrategy::SSD { size: 1024 * 1024 }),
        ),
        (
            "2MB_preread",
            Some(PreReadStrategy::SSD {
                size: 2 * 1024 * 1024,
            }),
        ),
    ];

    for (strategy_name, preread_strategy) in strategies {
        group.bench_with_input(
            BenchmarkId::new("preread_strategy", strategy_name),
            &preread_strategy,
            |b, &preread_strategy| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: preread_strategy.is_some(),
                            preread_strategy,
                            ..Default::default()
                        };
                        black_box(
                            engine
                                .copy_file_with_options(&source, &dest, options)
                                .await
                                .unwrap(),
                        )
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_file_reading,
    bench_file_writing,
    bench_buffer_operations,
    bench_copy_engines,
    bench_preread_strategies
);
criterion_main!(benches);
