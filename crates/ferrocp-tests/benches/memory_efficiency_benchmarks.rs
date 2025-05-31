//! Memory efficiency benchmarks for FerroCP
//!
//! This module provides detailed benchmarks for memory usage patterns,
//! allocation efficiency, and memory optimization strategies.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_io::{
    AdaptiveBuffer, BufferPool, BufferedCopyEngine, CopyEngine, MicroFileCopyEngine,
    MultiSizeBufferPool, ParallelCopyEngine,
};
use ferrocp_types::DeviceType;

/// Memory allocation tracker
struct AllocationTracker {
    allocations: Arc<AtomicUsize>,
    deallocations: Arc<AtomicUsize>,
    peak_memory: Arc<AtomicUsize>,
    current_memory: Arc<AtomicUsize>,
}

impl AllocationTracker {
    fn new() -> Self {
        Self {
            allocations: Arc::new(AtomicUsize::new(0)),
            deallocations: Arc::new(AtomicUsize::new(0)),
            peak_memory: Arc::new(AtomicUsize::new(0)),
            current_memory: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn track_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_memory.fetch_add(size, Ordering::Relaxed) + size;

        // Update peak if necessary
        let mut peak = self.peak_memory.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_memory.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    fn track_deallocation(&self, size: usize) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_memory.fetch_sub(size, Ordering::Relaxed);
    }

    fn get_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.allocations.load(Ordering::Relaxed),
            self.deallocations.load(Ordering::Relaxed),
            self.peak_memory.load(Ordering::Relaxed),
            self.current_memory.load(Ordering::Relaxed),
        )
    }

    fn reset(&self) {
        self.allocations.store(0, Ordering::Relaxed);
        self.deallocations.store(0, Ordering::Relaxed);
        self.peak_memory.store(0, Ordering::Relaxed);
        self.current_memory.store(0, Ordering::Relaxed);
    }
}

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = "A".repeat(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

/// Benchmark buffer pool memory efficiency
fn benchmark_buffer_pool_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool_efficiency");

    let pool_sizes = vec![
        (4, "4_buffers"),
        (8, "8_buffers"),
        (16, "16_buffers"),
        (32, "32_buffers"),
    ];

    let buffer_size = 64 * 1024; // 64KB buffers

    for (pool_size, pool_name) in &pool_sizes {
        group.throughput(Throughput::Elements(*pool_size as u64));

        // Benchmark BufferPool allocation/deallocation
        group.bench_with_input(
            BenchmarkId::new("buffer_pool_cycle", pool_name),
            pool_size,
            |b, &pool_size| {
                b.iter(|| {
                    let tracker = AllocationTracker::new();
                    let pool = BufferPool::new(pool_size, buffer_size);

                    // Simulate allocation/deallocation cycles
                    let mut buffers = Vec::new();
                    for _ in 0..pool_size {
                        tracker.track_allocation(buffer_size);
                        if let Some(buffer) = pool.get_buffer() {
                            buffers.push(buffer);
                        }
                    }

                    // Return buffers
                    for buffer in buffers {
                        tracker.track_deallocation(buffer_size);
                        pool.return_buffer(buffer);
                    }

                    black_box(tracker.get_stats());
                });
            },
        );

        // Benchmark MultiSizeBufferPool
        group.bench_with_input(
            BenchmarkId::new("multi_size_pool_cycle", pool_name),
            pool_size,
            |b, &pool_size| {
                b.iter(|| {
                    let tracker = AllocationTracker::new();
                    let pool = MultiSizeBufferPool::new();

                    // Test different buffer sizes
                    let sizes = vec![1024, 4096, 16384, 65536];
                    let mut buffers = Vec::new();

                    for &size in &sizes {
                        for _ in 0..(pool_size / 4) {
                            tracker.track_allocation(size);
                            if let Some(buffer) = pool.get_buffer(size) {
                                buffers.push((buffer, size));
                            }
                        }
                    }

                    // Return buffers
                    for (buffer, size) in buffers {
                        tracker.track_deallocation(size);
                        pool.return_buffer(buffer, size);
                    }

                    black_box(tracker.get_stats());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark adaptive buffer memory usage
fn benchmark_adaptive_buffer_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_buffer_memory");

    let scenarios = vec![
        (DeviceType::SSD, "SSD"),
        (DeviceType::HDD, "HDD"),
        (DeviceType::Network, "Network"),
        (DeviceType::RamDisk, "RamDisk"),
    ];

    for (device_type, device_name) in &scenarios {
        // Benchmark buffer adaptation memory efficiency
        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer_adaptation", device_name),
            device_type,
            |b, &device_type| {
                b.iter(|| {
                    let tracker = AllocationTracker::new();
                    let mut buffer = AdaptiveBuffer::new(device_type);

                    // Simulate adaptation cycles
                    for size in [1024, 4096, 16384, 65536, 262144] {
                        tracker.track_allocation(size);
                        buffer.resize(size);

                        // Simulate usage
                        let data = vec![0u8; size];
                        buffer.as_mut().copy_from_slice(&data);
                        buffer.clear();

                        tracker.track_deallocation(size);
                    }

                    black_box(tracker.get_stats());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark copy engine memory usage patterns
fn benchmark_copy_engine_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("copy_engine_memory_patterns");

    let file_sizes = vec![
        (1024, "1KB"),
        (16384, "16KB"),
        (1048576, "1MB"),
        (16777216, "16MB"),
    ];

    for (size, size_name) in &file_sizes {
        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark MicroFileCopyEngine memory usage
        if *size <= 4096 {
            group.bench_with_input(
                BenchmarkId::new("micro_engine_memory", size_name),
                size,
                |b, &size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.txt", size);
                        let dest = temp_dir.path().join("dest.txt");
                        let tracker = AllocationTracker::new();

                        rt.block_on(async {
                            tracker.track_allocation(size);
                            let mut engine = MicroFileCopyEngine::new();
                            let result = engine.copy_file(&source, &dest).await.unwrap();
                            tracker.track_deallocation(size);

                            black_box((result, tracker.get_stats()));
                        });
                    });
                },
            );
        }

        // Benchmark BufferedCopyEngine memory usage
        group.bench_with_input(
            BenchmarkId::new("buffered_engine_memory", size_name),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.txt", size);
                    let dest = temp_dir.path().join("dest.txt");
                    let tracker = AllocationTracker::new();

                    rt.block_on(async {
                        tracker.track_allocation(size);
                        let mut engine = BufferedCopyEngine::new();
                        let result = engine.copy_file(&source, &dest).await.unwrap();
                        tracker.track_deallocation(size);

                        black_box((result, tracker.get_stats()));
                    });
                });
            },
        );

        // Benchmark ParallelCopyEngine memory usage for large files
        if *size >= 16777216 {
            group.bench_with_input(
                BenchmarkId::new("parallel_engine_memory", size_name),
                size,
                |b, &size| {
                    b.iter(|| {
                        let temp_dir = TempDir::new().unwrap();
                        let source = create_test_file(&temp_dir, "source.txt", size);
                        let dest = temp_dir.path().join("dest.txt");
                        let tracker = AllocationTracker::new();

                        rt.block_on(async {
                            tracker.track_allocation(size);
                            let mut engine = ParallelCopyEngine::new();
                            let result = engine.copy_file(&source, &dest).await.unwrap();
                            tracker.track_deallocation(size);

                            black_box((result, tracker.get_stats()));
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark memory fragmentation patterns
fn benchmark_memory_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation");

    // Test different allocation patterns
    let patterns = vec![
        ("sequential", vec![1024, 2048, 4096, 8192, 16384]),
        ("random", vec![8192, 1024, 16384, 2048, 4096]),
        ("increasing", vec![1024, 2048, 4096, 8192, 16384]),
        ("decreasing", vec![16384, 8192, 4096, 2048, 1024]),
    ];

    for (pattern_name, sizes) in &patterns {
        group.bench_with_input(
            BenchmarkId::new("fragmentation_pattern", pattern_name),
            sizes,
            |b, sizes| {
                b.iter(|| {
                    let tracker = AllocationTracker::new();
                    let mut buffers = Vec::new();

                    // Allocate buffers in pattern
                    for &size in sizes {
                        tracker.track_allocation(size);
                        let buffer = AdaptiveBuffer::with_size(DeviceType::SSD, size);
                        buffers.push((buffer, size));
                    }

                    // Deallocate in reverse order (worst case for fragmentation)
                    for (buffer, size) in buffers.into_iter().rev() {
                        tracker.track_deallocation(size);
                        drop(buffer);
                    }

                    black_box(tracker.get_stats());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_buffer_pool_efficiency,
    benchmark_adaptive_buffer_memory,
    benchmark_copy_engine_memory_patterns,
    benchmark_memory_fragmentation
);

criterion_main!(benches);
