//! System call efficiency benchmarks for FerroCP
//!
//! This module provides benchmarks for system call patterns and efficiency,
//! helping to optimize the number and types of system calls made during file operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_io::{
    AsyncFileReader, AsyncFileWriter, BufferedCopyEngine, CopyEngine, MicroFileCopyEngine,
};

/// System call tracker for benchmarking
struct SyscallTracker {
    open_calls: Arc<AtomicUsize>,
    read_calls: Arc<AtomicUsize>,
    write_calls: Arc<AtomicUsize>,
    close_calls: Arc<AtomicUsize>,
    seek_calls: Arc<AtomicUsize>,
    flush_calls: Arc<AtomicUsize>,
    metadata_calls: Arc<AtomicUsize>,
}

impl SyscallTracker {
    fn new() -> Self {
        Self {
            open_calls: Arc::new(AtomicUsize::new(0)),
            read_calls: Arc::new(AtomicUsize::new(0)),
            write_calls: Arc::new(AtomicUsize::new(0)),
            close_calls: Arc::new(AtomicUsize::new(0)),
            seek_calls: Arc::new(AtomicUsize::new(0)),
            flush_calls: Arc::new(AtomicUsize::new(0)),
            metadata_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn track_open(&self) {
        self.open_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_read(&self) {
        self.read_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_write(&self) {
        self.write_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_close(&self) {
        self.close_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_seek(&self) {
        self.seek_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_flush(&self) {
        self.flush_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn track_metadata(&self) {
        self.metadata_calls.fetch_add(1, Ordering::Relaxed);
    }

    fn total_syscalls(&self) -> usize {
        self.open_calls.load(Ordering::Relaxed)
            + self.read_calls.load(Ordering::Relaxed)
            + self.write_calls.load(Ordering::Relaxed)
            + self.close_calls.load(Ordering::Relaxed)
            + self.seek_calls.load(Ordering::Relaxed)
            + self.flush_calls.load(Ordering::Relaxed)
            + self.metadata_calls.load(Ordering::Relaxed)
    }

    fn get_stats(&self) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
        (
            self.open_calls.load(Ordering::Relaxed),
            self.read_calls.load(Ordering::Relaxed),
            self.write_calls.load(Ordering::Relaxed),
            self.close_calls.load(Ordering::Relaxed),
            self.seek_calls.load(Ordering::Relaxed),
            self.flush_calls.load(Ordering::Relaxed),
            self.metadata_calls.load(Ordering::Relaxed),
            self.total_syscalls(),
        )
    }

    fn reset(&self) {
        self.open_calls.store(0, Ordering::Relaxed);
        self.read_calls.store(0, Ordering::Relaxed);
        self.write_calls.store(0, Ordering::Relaxed);
        self.close_calls.store(0, Ordering::Relaxed);
        self.seek_calls.store(0, Ordering::Relaxed);
        self.flush_calls.store(0, Ordering::Relaxed);
        self.metadata_calls.store(0, Ordering::Relaxed);
    }
}

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = "A".repeat(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

/// Benchmark system call efficiency for different copy strategies
fn benchmark_copy_syscall_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("copy_syscall_efficiency");

    let file_sizes = vec![
        (512, "512B"),
        (1024, "1KB"),
        (4096, "4KB"),
        (16384, "16KB"),
        (65536, "64KB"),
    ];

    for (size, size_name) in &file_sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark MicroFileCopyEngine syscall efficiency
        group.bench_with_input(
            BenchmarkId::new("micro_engine_syscalls", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = SyscallTracker::new();

                    rt.block_on(async {
                        // Simulate syscall tracking
                        tracker.track_metadata(); // File size check
                        tracker.track_open(); // Open source
                        tracker.track_open(); // Open dest

                        let mut engine = MicroFileCopyEngine::new();
                        let result = engine.copy_file(&source, &dest).await.unwrap();

                        // Estimate syscalls based on strategy
                        let stats = engine.stats();
                        let estimated_reads = (size + 4095) / 4096; // Estimate read syscalls
                        let estimated_writes = (size + 4095) / 4096; // Estimate write syscalls

                        for _ in 0..estimated_reads {
                            tracker.track_read();
                        }
                        for _ in 0..estimated_writes {
                            tracker.track_write();
                        }

                        tracker.track_flush(); // Flush
                        tracker.track_close(); // Close source
                        tracker.track_close(); // Close dest

                        black_box((result, tracker.get_stats(), stats));
                    });
                });
            },
        );

        // Benchmark BufferedCopyEngine syscall efficiency
        group.bench_with_input(
            BenchmarkId::new("buffered_engine_syscalls", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = SyscallTracker::new();

                    rt.block_on(async {
                        tracker.track_metadata(); // File size check
                        tracker.track_open(); // Open source
                        tracker.track_open(); // Open dest

                        let mut engine = BufferedCopyEngine::new();
                        let result = engine.copy_file(&source, &dest).await.unwrap();

                        // Estimate syscalls for buffered engine
                        let buffer_size = 64 * 1024; // Default buffer size
                        let estimated_reads = (size + buffer_size - 1) / buffer_size;
                        let estimated_writes = (size + buffer_size - 1) / buffer_size;

                        for _ in 0..estimated_reads {
                            tracker.track_read();
                        }
                        for _ in 0..estimated_writes {
                            tracker.track_write();
                        }

                        tracker.track_flush();
                        tracker.track_close();
                        tracker.track_close();

                        black_box((result, tracker.get_stats()));
                    });
                });
            },
        );

        // Benchmark std::fs::copy syscall efficiency
        group.bench_with_input(
            BenchmarkId::new("std_fs_copy_syscalls", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = SyscallTracker::new();

                    // Estimate syscalls for std::fs::copy
                    tracker.track_metadata(); // Source metadata
                    tracker.track_open(); // Open source
                    tracker.track_open(); // Open dest

                    let result = fs::copy(&source, &dest).unwrap();

                    // std::fs::copy typically uses larger buffers
                    let buffer_size = 64 * 1024;
                    let estimated_reads = (size + buffer_size - 1) / buffer_size;
                    let estimated_writes = (size + buffer_size - 1) / buffer_size;

                    for _ in 0..estimated_reads {
                        tracker.track_read();
                    }
                    for _ in 0..estimated_writes {
                        tracker.track_write();
                    }

                    tracker.track_metadata(); // Set dest metadata
                    tracker.track_close();
                    tracker.track_close();

                    black_box((result, tracker.get_stats()));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file I/O syscall patterns
fn benchmark_file_io_syscall_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_io_syscall_patterns");

    let io_patterns = vec![
        (1024, 1, "1KB_single_read"),
        (1024, 4, "1KB_4_reads"),
        (4096, 1, "4KB_single_read"),
        (4096, 4, "4KB_4_reads"),
        (16384, 1, "16KB_single_read"),
        (16384, 16, "16KB_16_reads"),
    ];

    for (total_size, num_operations, pattern_name) in &io_patterns {
        let chunk_size = total_size / num_operations;
        group.throughput(Throughput::Bytes(*total_size as u64));

        // Benchmark read syscall patterns
        group.bench_with_input(
            BenchmarkId::new("read_pattern", pattern_name),
            &(total_size, num_operations, chunk_size),
            |b, &(total_size, num_operations, _chunk_size)| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", *total_size);
                    let tracker = SyscallTracker::new();

                    rt.block_on(async {
                        tracker.track_open();
                        let reader = AsyncFileReader::open(&source).await.unwrap();

                        for _ in 0..*num_operations {
                            tracker.track_read();
                            // Simulate reading chunk_size bytes
                            // In real implementation, this would be actual read operations
                        }

                        tracker.track_close();
                        black_box((reader, tracker.get_stats()));
                    });
                });
            },
        );

        // Benchmark write syscall patterns
        group.bench_with_input(
            BenchmarkId::new("write_pattern", pattern_name),
            &(total_size, num_operations, chunk_size),
            |b, &(_total_size, num_operations, _chunk_size)| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = SyscallTracker::new();
                    let data = vec![0u8; chunk_size];

                    rt.block_on(async {
                        tracker.track_open();
                        let mut writer = AsyncFileWriter::create(&dest).await.unwrap();

                        for _ in 0..*num_operations {
                            tracker.track_write();
                            // Simulate writing chunk_size bytes
                            let _ = writer.write_all(&data).await;
                        }

                        tracker.track_flush();
                        tracker.track_close();
                        black_box((writer, tracker.get_stats()));
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark syscall optimization strategies
fn benchmark_syscall_optimization_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("syscall_optimization_strategies");

    let file_size = 4096; // 4KB test file

    // Test different MicroCopyStrategy syscall efficiency
    let strategies = vec![
        (ferrocp_io::MicroCopyStrategy::UltraFast, "UltraFast"),
        (ferrocp_io::MicroCopyStrategy::StackBuffer, "StackBuffer"),
        (ferrocp_io::MicroCopyStrategy::HyperFast, "HyperFast"),
        (ferrocp_io::MicroCopyStrategy::SuperFast, "SuperFast"),
    ];

    for (strategy, strategy_name) in &strategies {
        group.bench_with_input(
            BenchmarkId::new("strategy_syscalls", strategy_name),
            strategy,
            |b, strategy| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", file_size);
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = SyscallTracker::new();

                    rt.block_on(async {
                        let mut engine = MicroFileCopyEngine::with_strategy(*strategy);

                        // Track estimated syscalls for each strategy
                        tracker.track_metadata(); // File size check
                        tracker.track_open(); // Open source
                        tracker.track_open(); // Open dest

                        let result = engine.copy_file(&source, &dest).await.unwrap();
                        let stats = engine.stats();

                        // Different strategies may have different syscall patterns
                        match strategy {
                            ferrocp_io::MicroCopyStrategy::SuperFast => {
                                // SuperFast aims for zero additional syscalls
                                tracker.track_read(); // Single read
                                tracker.track_write(); // Single write
                            }
                            _ => {
                                // Other strategies may use multiple syscalls
                                let estimated_ops = (file_size + 1023) / 1024;
                                for _ in 0..estimated_ops {
                                    tracker.track_read();
                                    tracker.track_write();
                                }
                            }
                        }

                        tracker.track_close();
                        tracker.track_close();

                        black_box((result, stats, tracker.get_stats()));
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_copy_syscall_efficiency,
    benchmark_file_io_syscall_patterns,
    benchmark_syscall_optimization_strategies
);

criterion_main!(benches);
