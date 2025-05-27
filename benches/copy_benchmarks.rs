use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ferrocp::core::EACopy;
use ferrocp::config::Config;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Generate test data of specified size
fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Create a temporary file with test data
fn create_test_file(temp_dir: &TempDir, name: &str, size: usize) -> PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = generate_test_data(size);
    fs::write(&file_path, data).expect("Failed to write test file");
    file_path
}

/// Benchmark file copying with different sizes
fn bench_file_copy_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_copy_sizes");
    
    // Test different file sizes
    let sizes = vec![
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
        ("10MB", 10 * 1024 * 1024),
        ("100MB", 100 * 1024 * 1024),
    ];
    
    for (size_name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("ferrocp", size_name),
            &size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", size);
                    let dest = temp_dir.path().join("dest.dat");
                    
                    let eacopy = EACopy::new();
                    rt.block_on(async {
                        eacopy.copy_file(&source, &dest).await.unwrap()
                    })
                });
            },
        );
        
        // Compare with std::fs::copy
        group.bench_with_input(
            BenchmarkId::new("std_fs_copy", size_name),
            &size,
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

/// Benchmark different thread counts
fn bench_thread_counts(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("thread_counts");
    
    let file_size = 10 * 1024 * 1024; // 10MB
    group.throughput(Throughput::Bytes(file_size as u64));
    
    let thread_counts = vec![1, 2, 4, 8, 16];
    
    for thread_count in thread_counts {
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");
                    
                    let config = Config::new().with_thread_count(thread_count);
                    let eacopy = EACopy::with_config(config);
                    
                    rt.block_on(async {
                        eacopy.copy_file(&source, &dest).await.unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark compression levels
fn bench_compression_levels(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("compression_levels");
    
    let file_size = 1024 * 1024; // 1MB
    group.throughput(Throughput::Bytes(file_size as u64));
    
    let compression_levels = vec![0, 1, 3, 6, 9];
    
    for level in compression_levels {
        group.bench_with_input(
            BenchmarkId::new("compression", level),
            &level,
            |b, &level| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");
                    
                    let config = Config::new().with_compression_level(level);
                    let eacopy = EACopy::with_config(config);
                    
                    rt.block_on(async {
                        eacopy.copy_file(&source, &dest).await.unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark buffer sizes
fn bench_buffer_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("buffer_sizes");
    
    let file_size = 10 * 1024 * 1024; // 10MB
    group.throughput(Throughput::Bytes(file_size as u64));
    
    let buffer_sizes = vec![
        ("4KB", 4 * 1024),
        ("64KB", 64 * 1024),
        ("1MB", 1024 * 1024),
        ("8MB", 8 * 1024 * 1024),
        ("16MB", 16 * 1024 * 1024),
    ];
    
    for (size_name, buffer_size) in buffer_sizes {
        group.bench_with_input(
            BenchmarkId::new("buffer", size_name),
            &buffer_size,
            |b, &buffer_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source = create_test_file(&temp_dir, "source.dat", file_size);
                    let dest = temp_dir.path().join("dest.dat");
                    
                    let config = Config::new().with_buffer_size(buffer_size);
                    let eacopy = EACopy::with_config(config);
                    
                    rt.block_on(async {
                        eacopy.copy_file(&source, &dest).await.unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark directory copying
fn bench_directory_copy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("directory_copy");
    
    // Create test directory structures
    let file_counts = vec![10, 50, 100, 500];
    let file_size = 1024; // 1KB per file
    
    for file_count in file_counts {
        let total_size = file_count * file_size;
        group.throughput(Throughput::Bytes(total_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("files", file_count),
            &file_count,
            |b, &file_count| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let source_dir = temp_dir.path().join("source");
                    let dest_dir = temp_dir.path().join("dest");
                    
                    // Create source directory with files
                    fs::create_dir(&source_dir).unwrap();
                    for i in 0..file_count {
                        let file_path = source_dir.join(format!("file_{}.txt", i));
                        let data = generate_test_data(file_size);
                        fs::write(file_path, data).unwrap();
                    }
                    
                    let eacopy = EACopy::new();
                    rt.block_on(async {
                        eacopy.copy_directory(&source_dir, &dest_dir).await.unwrap()
                    })
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_file_copy_sizes,
    bench_thread_counts,
    bench_compression_levels,
    bench_buffer_sizes,
    bench_directory_copy
);
criterion_main!(benches);
