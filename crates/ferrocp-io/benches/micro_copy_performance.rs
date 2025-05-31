//! Performance benchmarks for MicroFileCopyEngine optimizations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferrocp_io::{MicroFileCopyEngine, MicroCopyStrategy, CopyEngine};
use std::fs;
use tempfile::TempDir;

fn bench_micro_copy_strategies(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("micro_copy_strategies");
    
    // Test different file sizes
    let file_sizes = vec![512, 1024, 2048, 4096]; // 512B, 1KB, 2KB, 4KB
    
    for size in file_sizes {
        // Benchmark UltraFast strategy
        group.bench_function(&format!("ultra_fast_{}B", size), |b| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = TempDir::new().unwrap();
                let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraFast);
                
                let source = temp_dir.path().join("source.txt");
                let content = "A".repeat(size);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join("dest.txt");
                
                black_box(engine.copy_file(&source, &destination).await.unwrap());
            });
        });
        
        // Benchmark StackBuffer strategy
        group.bench_function(&format!("stack_buffer_{}B", size), |b| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = TempDir::new().unwrap();
                let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::StackBuffer);
                
                let source = temp_dir.path().join("source.txt");
                let content = "A".repeat(size);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join("dest.txt");
                
                black_box(engine.copy_file(&source, &destination).await.unwrap());
            });
        });
        
        // Benchmark std::fs::copy for comparison
        group.bench_function(&format!("std_fs_copy_{}B", size), |b| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = TempDir::new().unwrap();
                
                let source = temp_dir.path().join("source.txt");
                let content = "A".repeat(size);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join("dest.txt");
                
                black_box(fs::copy(&source, &destination).unwrap());
            });
        });
    }
    
    group.finish();
}

fn bench_empty_file_handling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("empty_file_handling");
    
    // Benchmark UltraFast strategy with empty files
    group.bench_function("ultra_fast_empty", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraFast);
            
            let source = temp_dir.path().join("empty.txt");
            fs::write(&source, "").unwrap(); // Empty file
            
            let destination = temp_dir.path().join("dest_empty.txt");
            
            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });
    
    // Benchmark StackBuffer strategy with empty files
    group.bench_function("stack_buffer_empty", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::StackBuffer);
            
            let source = temp_dir.path().join("empty.txt");
            fs::write(&source, "").unwrap(); // Empty file
            
            let destination = temp_dir.path().join("dest_empty.txt");
            
            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });
    
    // Benchmark std::fs::copy with empty files
    group.bench_function("std_fs_copy_empty", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            
            let source = temp_dir.path().join("empty.txt");
            fs::write(&source, "").unwrap(); // Empty file
            
            let destination = temp_dir.path().join("dest_empty.txt");
            
            black_box(fs::copy(&source, &destination).unwrap());
        });
    });
    
    group.finish();
}

fn bench_batch_micro_copy(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("batch_micro_copy_ultra_fast", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraFast);
            
            // Create 100 small files
            for i in 0..100 {
                let source = temp_dir.path().join(format!("source_{}.txt", i));
                let content = format!("Content for file {}", i);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join(format!("dest_{}.txt", i));
                
                black_box(engine.copy_file(&source, &destination).await.unwrap());
            }
        });
    });
    
    c.bench_function("batch_micro_copy_stack_buffer", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::StackBuffer);
            
            // Create 100 small files
            for i in 0..100 {
                let source = temp_dir.path().join(format!("source_{}.txt", i));
                let content = format!("Content for file {}", i);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join(format!("dest_{}.txt", i));
                
                black_box(engine.copy_file(&source, &destination).await.unwrap());
            }
        });
    });
    
    c.bench_function("batch_std_fs_copy", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            
            // Create 100 small files
            for i in 0..100 {
                let source = temp_dir.path().join(format!("source_{}.txt", i));
                let content = format!("Content for file {}", i);
                fs::write(&source, &content).unwrap();
                
                let destination = temp_dir.path().join(format!("dest_{}.txt", i));
                
                black_box(fs::copy(&source, &destination).unwrap());
            }
        });
    });
}

fn bench_1kb_file_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("1kb_file_performance");
    
    // Focus on 1KB files - the target for 25% improvement over std::fs::copy
    group.bench_function("ferrocp_ultra_fast_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraFast);
            
            let source = temp_dir.path().join("1kb_file.txt");
            let content = "X".repeat(1024); // Exactly 1KB
            fs::write(&source, &content).unwrap();
            
            let destination = temp_dir.path().join("dest_1kb.txt");
            
            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });
    
    group.bench_function("ferrocp_stack_buffer_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::StackBuffer);

            let source = temp_dir.path().join("1kb_file.txt");
            let content = "X".repeat(1024); // Exactly 1KB
            fs::write(&source, &content).unwrap();

            let destination = temp_dir.path().join("dest_1kb.txt");

            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });

    group.bench_function("ferrocp_super_fast_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::SuperFast);

            let source = temp_dir.path().join("1kb_file.txt");
            let content = "X".repeat(1024); // Exactly 1KB
            fs::write(&source, &content).unwrap();

            let destination = temp_dir.path().join("dest_1kb.txt");

            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });

    group.bench_function("ferrocp_ultra_optimized_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut engine = MicroFileCopyEngine::with_strategy(MicroCopyStrategy::UltraOptimized);

            let source = temp_dir.path().join("1kb_file.txt");
            let content = "X".repeat(1024); // Exactly 1KB
            fs::write(&source, &content).unwrap();

            let destination = temp_dir.path().join("dest_1kb.txt");

            black_box(engine.copy_file(&source, &destination).await.unwrap());
        });
    });

    group.bench_function("std_fs_copy_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            
            let source = temp_dir.path().join("1kb_file.txt");
            let content = "X".repeat(1024); // Exactly 1KB
            fs::write(&source, &content).unwrap();
            
            let destination = temp_dir.path().join("dest_1kb.txt");
            
            black_box(fs::copy(&source, &destination).unwrap());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_micro_copy_strategies,
    bench_empty_file_handling,
    bench_batch_micro_copy,
    bench_1kb_file_performance
);
criterion_main!(benches);
