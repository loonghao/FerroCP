//! Concurrency performance benchmarks for FerroCP
//!
//! This module provides comprehensive benchmarks for multi-threaded performance,
//! resource contention analysis, and scalability testing across 1-32 threads.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::runtime::Runtime;

use ferrocp_io::{BufferedCopyEngine, CopyEngine, MicroFileCopyEngine};

/// Concurrency metrics tracker
#[derive(Debug, Clone)]
struct ConcurrencyMetrics {
    /// Number of threads used
    thread_count: usize,
    /// Total operations completed
    operations_completed: Arc<AtomicUsize>,
    /// Total bytes processed
    bytes_processed: Arc<AtomicUsize>,
    /// Lock contention events
    lock_contentions: Arc<AtomicUsize>,
    /// Memory allocation events
    memory_allocations: Arc<AtomicUsize>,
    /// Peak memory usage
    peak_memory_usage: Arc<AtomicUsize>,
    /// Start time
    start_time: Instant,
}

impl ConcurrencyMetrics {
    fn new(thread_count: usize) -> Self {
        Self {
            thread_count,
            operations_completed: Arc::new(AtomicUsize::new(0)),
            bytes_processed: Arc::new(AtomicUsize::new(0)),
            lock_contentions: Arc::new(AtomicUsize::new(0)),
            memory_allocations: Arc::new(AtomicUsize::new(0)),
            peak_memory_usage: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    fn record_operation(&self, bytes: usize) {
        self.operations_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    fn record_lock_contention(&self) {
        self.lock_contentions.fetch_add(1, Ordering::Relaxed);
    }

    fn record_memory_allocation(&self, size: usize) {
        self.memory_allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.peak_memory_usage.load(Ordering::Relaxed);
        if size > current {
            self.peak_memory_usage.store(size, Ordering::Relaxed);
        }
    }

    fn get_throughput_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let mb_processed = self.bytes_processed.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
        mb_processed / elapsed
    }

    fn get_scalability_factor(&self) -> f64 {
        let operations = self.operations_completed.load(Ordering::Relaxed) as f64;
        operations / self.thread_count as f64
    }

    fn get_contention_ratio(&self) -> f64 {
        let contentions = self.lock_contentions.load(Ordering::Relaxed) as f64;
        let operations = self.operations_completed.load(Ordering::Relaxed) as f64;
        if operations > 0.0 {
            contentions / operations
        } else {
            0.0
        }
    }
}

/// Resource contention simulator
struct ResourceContentionSimulator {
    shared_resource: Arc<Mutex<Vec<u8>>>,
    metrics: ConcurrencyMetrics,
}

impl ResourceContentionSimulator {
    fn new(thread_count: usize, resource_size: usize) -> Self {
        Self {
            shared_resource: Arc::new(Mutex::new(vec![0u8; resource_size])),
            metrics: ConcurrencyMetrics::new(thread_count),
        }
    }

    fn simulate_contention(&self, operations_per_thread: usize) {
        let mut handles = Vec::new();
        
        for _ in 0..self.metrics.thread_count {
            let resource = Arc::clone(&self.shared_resource);
            let metrics = self.metrics.clone();
            
            let handle = thread::spawn(move || {
                for _ in 0..operations_per_thread {
                    let start = Instant::now();
                    
                    // Simulate lock contention
                    match resource.try_lock() {
                        Ok(mut data) => {
                            // Simulate work with shared resource
                            data[0] = data[0].wrapping_add(1);
                            metrics.record_operation(data.len());
                        }
                        Err(_) => {
                            // Lock contention detected
                            metrics.record_lock_contention();
                            
                            // Wait and try again
                            thread::sleep(Duration::from_micros(1));
                            if let Ok(mut data) = resource.lock() {
                                data[0] = data[0].wrapping_add(1);
                                metrics.record_operation(data.len());
                            }
                        }
                    }
                    
                    // Simulate memory allocation
                    let temp_data = vec![0u8; 1024];
                    metrics.record_memory_allocation(temp_data.len());
                    black_box(temp_data);
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    fn get_metrics(&self) -> &ConcurrencyMetrics {
        &self.metrics
    }
}

/// Helper function to create test file
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> std::path::PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = "A".repeat(size);
    fs::write(&file_path, data).unwrap();
    file_path
}

/// Benchmark multi-threaded file copy performance
fn benchmark_multithreaded_copy_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("multithreaded_copy_performance");

    // Test different thread counts
    let thread_counts = vec![1, 2, 4, 8, 16, 32];
    let file_size = 1024 * 1024; // 1MB per file
    let files_per_thread = 10;

    for thread_count in &thread_counts {
        group.throughput(Throughput::Bytes((file_size * files_per_thread * thread_count) as u64));

        // Benchmark MicroFileCopyEngine concurrency
        group.bench_with_input(
            BenchmarkId::new("micro_engine_concurrent", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let metrics = ConcurrencyMetrics::new(thread_count);

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Create all source files first
                        let sources: Vec<_> = (0..thread_count)
                            .map(|i| create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size))
                            .collect();

                        for i in 0..thread_count {
                            let source = sources[i].clone();
                            let temp_dir_path = temp_dir.path().to_path_buf();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = MicroFileCopyEngine::new();
                                for j in 0..files_per_thread {
                                    let file_dest = temp_dir_path.join(format!("dest_{}_{}.txt", i, j));
                                    let result = engine.copy_file(&source, &file_dest).await.unwrap();
                                    metrics_clone.record_operation(result.bytes_copied as usize);
                                }
                            });

                            handles.push(handle);
                        }

                        // Wait for all tasks to complete
                        for handle in handles {
                            handle.await.unwrap();
                        }

                        black_box((metrics.get_throughput_mbps(), metrics.get_scalability_factor()));
                    });
                });
            },
        );

        // Benchmark BufferedCopyEngine concurrency
        group.bench_with_input(
            BenchmarkId::new("buffered_engine_concurrent", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let metrics = ConcurrencyMetrics::new(thread_count);

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Create all source files first
                        let sources: Vec<_> = (0..thread_count)
                            .map(|i| create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size))
                            .collect();

                        for i in 0..thread_count {
                            let source = sources[i].clone();
                            let temp_dir_path = temp_dir.path().to_path_buf();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = BufferedCopyEngine::new();
                                for j in 0..files_per_thread {
                                    let file_dest = temp_dir_path.join(format!("dest_{}_{}.txt", i, j));
                                    let result = engine.copy_file(&source, &file_dest).await.unwrap();
                                    metrics_clone.record_operation(result.bytes_copied as usize);
                                }
                            });

                            handles.push(handle);
                        }

                        // Wait for all tasks to complete
                        for handle in handles {
                            handle.await.unwrap();
                        }

                        black_box((metrics.get_throughput_mbps(), metrics.get_scalability_factor()));
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark resource contention patterns
fn benchmark_resource_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_contention");

    let thread_counts = vec![1, 2, 4, 8, 16, 32];
    let operations_per_thread = 1000;
    let resource_size = 1024; // 1KB shared resource

    for thread_count in &thread_counts {
        group.throughput(Throughput::Elements((operations_per_thread * thread_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("lock_contention", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let simulator = ResourceContentionSimulator::new(thread_count, resource_size);
                    simulator.simulate_contention(operations_per_thread);
                    
                    let metrics = simulator.get_metrics();
                    black_box((
                        metrics.get_throughput_mbps(),
                        metrics.get_contention_ratio(),
                        metrics.get_scalability_factor(),
                    ));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns in concurrent environments
fn benchmark_concurrent_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_memory_usage");

    let thread_counts = vec![1, 4, 8, 16];
    let file_size = 512 * 1024; // 512KB per file
    let files_per_thread = 5;

    for thread_count in &thread_counts {
        group.throughput(Throughput::Bytes((file_size * files_per_thread * thread_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("memory_allocation_pattern", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let metrics = ConcurrencyMetrics::new(thread_count);

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Create all source files first
                        let sources: Vec<_> = (0..thread_count)
                            .map(|i| create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size))
                            .collect();

                        let temp_dir_path = temp_dir.path().to_path_buf();

                        for i in 0..thread_count {
                            let source = sources[i].clone();
                            let temp_dir_path_clone = temp_dir_path.clone();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = BufferedCopyEngine::new();

                                for j in 0..files_per_thread {
                                    // Simulate memory allocation tracking
                                    metrics_clone.record_memory_allocation(file_size);

                                    let dest = temp_dir_path_clone.join(format!("dest_{}_{}.txt", i, j));
                                    let result = engine.copy_file(&source, &dest).await.unwrap();
                                    metrics_clone.record_operation(result.bytes_copied as usize);

                                    // Simulate memory deallocation
                                    metrics_clone.record_memory_allocation(0);
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        black_box((
                            metrics.peak_memory_usage.load(Ordering::Relaxed),
                            metrics.memory_allocations.load(Ordering::Relaxed),
                            metrics.get_throughput_mbps(),
                        ));
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark thread scalability analysis
fn benchmark_thread_scalability(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("thread_scalability");

    let thread_counts = vec![1, 2, 4, 6, 8, 12, 16, 20, 24, 28, 32];
    let file_size = 256 * 1024; // 256KB per file
    let total_work = 32 * 1024 * 1024; // 32MB total work

    for thread_count in &thread_counts {
        let files_per_thread = (total_work / file_size) / thread_count;
        group.throughput(Throughput::Bytes(total_work as u64));

        group.bench_with_input(
            BenchmarkId::new("scalability_analysis", thread_count),
            &(*thread_count, files_per_thread),
            |b, &(thread_count, files_per_thread)| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let metrics = ConcurrencyMetrics::new(thread_count);
                    let start_time = Instant::now();

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Create all source files first
                        let sources: Vec<_> = (0..thread_count)
                            .map(|i| create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size))
                            .collect();

                        for i in 0..thread_count {
                            let source = sources[i].clone();
                            let temp_dir_path = temp_dir.path().to_path_buf();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = BufferedCopyEngine::new();

                                for j in 0..files_per_thread {
                                    let dest = temp_dir_path.join(format!("dest_{}_{}.txt", i, j));
                                    let result = engine.copy_file(&source, &dest).await.unwrap();
                                    metrics_clone.record_operation(result.bytes_copied as usize);
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        let elapsed = start_time.elapsed();
                        let throughput = total_work as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0);
                        let efficiency = throughput / thread_count as f64;

                        black_box((throughput, efficiency, metrics.get_scalability_factor()));
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark I/O resource competition
fn benchmark_io_resource_competition(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("io_resource_competition");

    let thread_counts = vec![1, 2, 4, 8, 16];
    let file_size = 1024 * 1024; // 1MB per file
    let files_per_thread = 3;

    for thread_count in &thread_counts {
        group.throughput(Throughput::Bytes((file_size * files_per_thread * thread_count) as u64));

        // Test same source file (read competition)
        group.bench_with_input(
            BenchmarkId::new("read_competition", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "shared_source.txt", file_size);
                    let metrics = ConcurrencyMetrics::new(thread_count);
                    let temp_dir_path = temp_dir.path().to_path_buf();

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        for i in 0..thread_count {
                            let source_clone = source.clone();
                            let temp_dir_path_clone = temp_dir_path.clone();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = BufferedCopyEngine::new();

                                for j in 0..files_per_thread {
                                    let dest = temp_dir_path_clone.join(format!("dest_{}_{}.txt", i, j));

                                    // Simulate I/O contention detection
                                    let _start = Instant::now();
                                    let result = engine.copy_file(&source_clone, &dest).await.unwrap();
                                    let elapsed = _start.elapsed();

                                    // If operation took longer than expected, record contention
                                    if elapsed > Duration::from_millis(100) {
                                        metrics_clone.record_lock_contention();
                                    }

                                    metrics_clone.record_operation(result.bytes_copied as usize);
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        black_box((
                            metrics.get_throughput_mbps(),
                            metrics.get_contention_ratio(),
                        ));
                    });
                });
            },
        );

        // Test same destination directory (write competition)
        group.bench_with_input(
            BenchmarkId::new("write_competition", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let metrics = ConcurrencyMetrics::new(thread_count);

                    rt.block_on(async {
                        let mut handles = Vec::new();

                        // Create all source files first
                        let sources: Vec<_> = (0..thread_count)
                            .map(|i| create_test_file(&temp_dir, &format!("source_{}.txt", i), file_size))
                            .collect();

                        let temp_dir_path = temp_dir.path().to_path_buf();

                        for i in 0..thread_count {
                            let source = sources[i].clone();
                            let temp_dir_path_clone = temp_dir_path.clone();
                            let metrics_clone = metrics.clone();

                            let handle = tokio::spawn(async move {
                                let mut engine = BufferedCopyEngine::new();

                                for j in 0..files_per_thread {
                                    // All threads write to same directory (potential filesystem contention)
                                    let dest = temp_dir_path_clone.join(format!("shared_dest_{}_{}.txt", i, j));

                                    let _start = Instant::now();
                                    let result = engine.copy_file(&source, &dest).await.unwrap();
                                    let elapsed = _start.elapsed();

                                    // Detect write contention
                                    if elapsed > Duration::from_millis(100) {
                                        metrics_clone.record_lock_contention();
                                    }

                                    metrics_clone.record_operation(result.bytes_copied as usize);
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }

                        black_box((
                            metrics.get_throughput_mbps(),
                            metrics.get_contention_ratio(),
                        ));
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_multithreaded_copy_performance,
    benchmark_resource_contention,
    benchmark_concurrent_memory_usage,
    benchmark_thread_scalability,
    benchmark_io_resource_competition
);

criterion_main!(benches);
