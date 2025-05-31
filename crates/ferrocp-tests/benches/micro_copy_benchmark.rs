//! Micro file copy engine benchmarks
//!
//! These benchmarks specifically test the performance of the MicroFileCopyEngine
//! against standard file copy operations for small files.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::time::Instant;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_io::{BufferedCopyEngine, CopyEngine, MicroFileCopyEngine, ParallelCopyEngine};

/// Memory usage tracker for benchmarks
struct MemoryTracker {
    start_memory: usize,
    peak_memory: usize,
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            start_memory: Self::get_memory_usage(),
            peak_memory: 0,
        }
    }

    fn update_peak(&mut self) {
        let current = Self::get_memory_usage();
        if current > self.peak_memory {
            self.peak_memory = current;
        }
    }

    fn memory_delta(&self) -> usize {
        self.peak_memory.saturating_sub(self.start_memory)
    }

    #[cfg(target_os = "windows")]
    fn get_memory_usage() -> usize {
        // Simplified memory tracking for Windows
        // In a real implementation, this would use Windows APIs
        0
    }

    #[cfg(not(target_os = "windows"))]
    fn get_memory_usage() -> usize {
        // Simplified memory tracking for Unix-like systems
        // In a real implementation, this would use /proc/self/status or similar
        0
    }
}

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = "A".repeat(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

/// Benchmark micro file copy engine vs buffered copy engine
fn benchmark_micro_vs_buffered(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("micro_vs_buffered");

    // Test different small file sizes including critical 1KB target
    let sizes = vec![
        (100, "100B"),
        (500, "500B"),
        (1024, "1KB"), // Critical target size
        (2048, "2KB"),
        (4096, "4KB"), // Threshold boundary
    ];

    for (size, size_name) in &sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark MicroFileCopyEngine
        group.bench_with_input(
            BenchmarkId::new("micro_engine", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = MicroFileCopyEngine::new();
                        let result = engine.copy_file(&source, &dest).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Benchmark BufferedCopyEngine
        group.bench_with_input(
            BenchmarkId::new("buffered_engine", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    rt.block_on(async {
                        let mut engine = BufferedCopyEngine::new();
                        let result = engine.copy_file(&source, &dest).await.unwrap();
                        black_box(result);
                    })
                });
            },
        );

        // Benchmark std::fs::copy
        group.bench_with_input(
            BenchmarkId::new("std_fs_copy", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");

                    black_box(fs::copy(&source, &dest).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark micro file copy engine throughput
fn benchmark_micro_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("micro_throughput");

    // Test with 1KB files to measure throughput
    let file_size = 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    group.bench_function("micro_engine_1kb", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = MicroFileCopyEngine::new();
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);
            })
        });
    });

    group.finish();
}

/// Benchmark batch micro file operations
fn benchmark_micro_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("micro_batch");

    // Test copying multiple small files
    let file_count = 10;
    let file_size = 512; // 512 bytes each
    group.throughput(Throughput::Bytes((file_count * file_size) as u64));

    group.bench_function("micro_engine_batch", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();

            rt.block_on(async {
                let mut engine = MicroFileCopyEngine::new();

                for i in 0..file_count {
                    let source =
                        create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size);
                    let dest = temp_dir.path().join(format!("dest_{}.txt", i));

                    let result = engine.copy_file(&source, &dest).await.unwrap();
                    black_box(result);
                }

                // Check final statistics
                let stats = engine.stats();
                black_box(stats);
            })
        });
    });

    group.bench_function("buffered_engine_batch", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();

            rt.block_on(async {
                let mut engine = BufferedCopyEngine::new();

                for i in 0..file_count {
                    let source =
                        create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size);
                    let dest = temp_dir.path().join(format!("dest_{}.txt", i));

                    let result = engine.copy_file(&source, &dest).await.unwrap();
                    black_box(result);
                }
            })
        });
    });

    group.finish();
}

/// Benchmark different micro copy strategies
fn benchmark_micro_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("micro_strategies");

    // Focus on 1KB files for strategy comparison
    let file_size = 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    // Benchmark UltraFast strategy
    group.bench_function("ultra_fast_1kb", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine =
                    MicroFileCopyEngine::with_strategy(ferrocp_io::MicroCopyStrategy::UltraFast);
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);

                // Check optimization statistics
                let stats = engine.stats();
                black_box(stats);
            })
        });
    });

    // Benchmark StackBuffer strategy
    group.bench_function("stack_buffer_1kb", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine =
                    MicroFileCopyEngine::with_strategy(ferrocp_io::MicroCopyStrategy::StackBuffer);
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);

                // Check optimization statistics
                let stats = engine.stats();
                black_box(stats);
            })
        });
    });

    // Benchmark HyperFast strategy
    group.bench_function("hyper_fast_1kb", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine =
                    MicroFileCopyEngine::with_strategy(ferrocp_io::MicroCopyStrategy::HyperFast);
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);

                // Check optimization statistics
                let stats = engine.stats();
                black_box(stats);
            })
        });
    });

    // Benchmark SuperFast strategy (new zero-syscall optimization)
    group.bench_function("super_fast_1kb", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine =
                    MicroFileCopyEngine::with_strategy(ferrocp_io::MicroCopyStrategy::SuperFast);
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);

                // Check optimization statistics
                let stats = engine.stats();
                black_box(stats);
            })
        });
    });

    group.finish();
}

/// Benchmark system call efficiency
fn benchmark_syscall_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("syscall_efficiency");

    // Test with 1KB files to measure syscall overhead
    let file_size = 1024;
    group.throughput(Throughput::Bytes(file_size as u64));

    group.bench_function("micro_engine_syscalls", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let source = create_test_file(&temp_dir, "source.txt", file_size);
            let dest = temp_dir.path().join("dest.txt");

            rt.block_on(async {
                let mut engine = MicroFileCopyEngine::new();
                let result = engine.copy_file(&source, &dest).await.unwrap();
                black_box(result);

                // Measure syscall efficiency
                let avg_syscalls = engine.average_syscalls_per_file();
                black_box(avg_syscalls);
            })
        });
    });

    group.finish();
}

/// Benchmark detailed file size performance analysis (1B to 1GB range)
fn benchmark_file_size_analysis(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_size_analysis");

    // Extended file size range for detailed analysis
    let sizes = vec![
        (1, "1B"),
        (10, "10B"),
        (100, "100B"),
        (1024, "1KB"),
        (4096, "4KB"),
        (16384, "16KB"),
        (65536, "64KB"),
        (262144, "256KB"),
        (1048576, "1MB"),
        (16777216, "16MB"),
        (67108864, "64MB"),
        (268435456, "256MB"),
        (1073741824, "1GB"),
    ];

    for (size, size_name) in &sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Test MicroFileCopyEngine for small files
        if *size <= 4096 {
            group.bench_with_input(
                BenchmarkId::new("micro_detailed", size_name),
                size,
                |b, &size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.txt", size);
                        let dest = temp_dir.path().join("dest.txt");
                        let mut memory_tracker = MemoryTracker::new();
                        let start_time = Instant::now();

                        rt.block_on(async {
                            let mut engine = MicroFileCopyEngine::new();
                            memory_tracker.update_peak();
                            let result = engine.copy_file(&source, &dest).await.unwrap();
                            memory_tracker.update_peak();
                            black_box((
                                result,
                                memory_tracker.memory_delta(),
                                start_time.elapsed(),
                            ));
                        })
                    });
                },
            );
        }

        // Test BufferedCopyEngine for medium files
        if *size >= 1024 && *size <= 67108864 {
            group.bench_with_input(
                BenchmarkId::new("buffered_detailed", size_name),
                size,
                |b, &size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.txt", size);
                        let dest = temp_dir.path().join("dest.txt");
                        let mut memory_tracker = MemoryTracker::new();
                        let start_time = Instant::now();

                        rt.block_on(async {
                            let mut engine = BufferedCopyEngine::new();
                            memory_tracker.update_peak();
                            let result = engine.copy_file(&source, &dest).await.unwrap();
                            memory_tracker.update_peak();
                            black_box((
                                result,
                                memory_tracker.memory_delta(),
                                start_time.elapsed(),
                            ));
                        })
                    });
                },
            );
        }

        // Test ParallelCopyEngine for large files
        if *size >= 16777216 {
            group.bench_with_input(
                BenchmarkId::new("parallel_detailed", size_name),
                size,
                |b, &size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.txt", size);
                        let dest = temp_dir.path().join("dest.txt");
                        let mut memory_tracker = MemoryTracker::new();
                        let start_time = Instant::now();

                        rt.block_on(async {
                            let mut engine = ParallelCopyEngine::new();
                            memory_tracker.update_peak();
                            let result = engine.copy_file(&source, &dest).await.unwrap();
                            memory_tracker.update_peak();
                            black_box((
                                result,
                                memory_tracker.memory_delta(),
                                start_time.elapsed(),
                            ));
                        })
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark memory efficiency across different engines
fn benchmark_memory_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_efficiency");

    let test_sizes = vec![
        (1024, "1KB"),
        (16384, "16KB"),
        (1048576, "1MB"),
        (16777216, "16MB"),
    ];

    for (size, size_name) in &test_sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Memory efficiency test for each engine
        group.bench_with_input(
            BenchmarkId::new("memory_micro", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");
                    let mut memory_tracker = MemoryTracker::new();

                    rt.block_on(async {
                        let mut engine = MicroFileCopyEngine::new();
                        memory_tracker.update_peak();
                        let result = engine.copy_file(&source, &dest).await.unwrap();
                        memory_tracker.update_peak();

                        // Return both performance and memory metrics
                        black_box((result.bytes_copied, memory_tracker.memory_delta()));
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_micro_vs_buffered,
    benchmark_micro_throughput,
    benchmark_micro_batch,
    benchmark_micro_strategies,
    benchmark_syscall_efficiency,
    benchmark_file_size_analysis,
    benchmark_memory_efficiency
);

criterion_main!(benches);
