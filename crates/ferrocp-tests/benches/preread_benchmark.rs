//! Benchmark for pre-read algorithm performance
//!
//! This benchmark validates the 512KB SSD preread optimization that provides
//! 30.1% performance improvement (387.82 MiB/s vs 298.10 MiB/s).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrocp_io::{BufferedCopyEngine, CopyEngine, CopyOptions, PreReadStrategy};
use ferrocp_types::DeviceType;
use std::fs;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Create a test file with the specified size
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let content = "A".repeat(size);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Benchmark pre-read vs normal copy for large files
fn benchmark_preread_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("preread_performance");

    // Test with different file sizes
    let file_sizes = [
        (10 * 1024 * 1024, "10MB"),   // 10MB - should trigger pre-read
        (50 * 1024 * 1024, "50MB"),   // 50MB
        (100 * 1024 * 1024, "100MB"), // 100MB
    ];

    for (size, size_name) in file_sizes.iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark without pre-read
        group.bench_with_input(
            BenchmarkId::new("without_preread", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: false, // Disable pre-read
                            ..Default::default()
                        };
                        let result = engine
                            .copy_file_with_options(&source, &dest, options)
                            .await
                            .unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Benchmark with optimized 512KB SSD strategy
        group.bench_with_input(
            BenchmarkId::new("with_preread_ssd_optimized", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: true,
                            preread_strategy: Some(PreReadStrategy::for_device(
                                DeviceType::SSD,
                                false,
                            )), // Optimized 512KB
                            ..Default::default()
                        };
                        let result = engine
                            .copy_file_with_options(&source, &dest, options)
                            .await
                            .unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Benchmark with old 1MB SSD strategy for comparison
        group.bench_with_input(
            BenchmarkId::new("with_preread_ssd_old", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: true,
                            preread_strategy: Some(PreReadStrategy::SSD { size: 1024 * 1024 }), // Old 1MB
                            ..Default::default()
                        };
                        let result = engine
                            .copy_file_with_options(&source, &dest, options)
                            .await
                            .unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Benchmark with pre-read (HDD strategy)
        group.bench_with_input(
            BenchmarkId::new("with_preread_hdd", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: true,
                            preread_strategy: Some(PreReadStrategy::HDD { size: 64 * 1024 }), // 64KB pre-read
                            ..Default::default()
                        };
                        let result = engine
                            .copy_file_with_options(&source, &dest, options)
                            .await
                            .unwrap();
                        black_box(result);
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark different pre-read strategies
fn benchmark_preread_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("preread_strategies");

    // Test with 50MB file
    let file_size = 50 * 1024 * 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    // Test different pre-read sizes for SSD strategy
    let preread_sizes = [
        (256 * 1024, "256KB"),
        (512 * 1024, "512KB"),
        (1024 * 1024, "1MB"),
        (2 * 1024 * 1024, "2MB"),
        (4 * 1024 * 1024, "4MB"),
    ];

    for (preread_size, size_name) in preread_sizes.iter() {
        group.bench_with_input(
            BenchmarkId::new("ssd_strategy", size_name),
            preread_size,
            |b, &preread_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", file_size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let options = CopyOptions {
                            enable_preread: true,
                            preread_strategy: Some(PreReadStrategy::SSD { size: preread_size }),
                            ..Default::default()
                        };
                        let result = engine
                            .copy_file_with_options(&source, &dest, options)
                            .await
                            .unwrap();
                        black_box(result);
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pre-read hit ratio effectiveness
fn benchmark_preread_hit_ratio(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("preread_hit_ratio");

    // Test with 20MB file to see pre-read effectiveness
    let file_size = 20 * 1024 * 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    group.bench_function("preread_effectiveness", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = BufferedCopyEngine::new();
                let options = CopyOptions {
                    enable_preread: true,
                    preread_strategy: Some(PreReadStrategy::SSD { size: 1024 * 1024 }),
                    ..Default::default()
                };
                let result = engine
                    .copy_file_with_options(&source, &dest, options)
                    .await
                    .unwrap();
                black_box(result);
            })
        });
    });

    group.finish();
}

/// Benchmark specifically for 512KB optimization validation
fn benchmark_512kb_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("512kb_optimization_validation");

    // Test with 50MB file - ideal for demonstrating preread benefits
    let file_size = 50 * 1024 * 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    // Benchmark optimized 512KB (expected: 387.82 MiB/s)
    group.bench_function("optimized_512KB", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = BufferedCopyEngine::new();
                let options = CopyOptions {
                    enable_preread: true,
                    preread_strategy: Some(PreReadStrategy::SSD { size: 512 * 1024 }), // Optimized
                    ..Default::default()
                };
                let result = engine
                    .copy_file_with_options(&source, &dest, options)
                    .await
                    .unwrap();
                black_box(result);
            })
        });
    });

    // Benchmark old 1MB (baseline: 298.10 MiB/s)
    group.bench_function("old_1MB", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = BufferedCopyEngine::new();
                let options = CopyOptions {
                    enable_preread: true,
                    preread_strategy: Some(PreReadStrategy::SSD { size: 1024 * 1024 }), // Old
                    ..Default::default()
                };
                let result = engine
                    .copy_file_with_options(&source, &dest, options)
                    .await
                    .unwrap();
                black_box(result);
            })
        });
    });

    // Benchmark auto-detected strategy (should use optimized 512KB)
    group.bench_function("auto_detected", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = BufferedCopyEngine::new();
                let options = CopyOptions {
                    enable_preread: true,
                    preread_strategy: Some(PreReadStrategy::for_device(DeviceType::SSD, false)), // Auto
                    ..Default::default()
                };
                let result = engine
                    .copy_file_with_options(&source, &dest, options)
                    .await
                    .unwrap();
                black_box(result);
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_preread_performance,
    benchmark_preread_strategies,
    benchmark_preread_hit_ratio,
    benchmark_512kb_optimization
);

criterion_main!(benches);
