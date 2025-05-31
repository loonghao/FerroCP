//! Performance benchmarks for engine selector and dynamic threshold adjustment

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferrocp_engine::selector::{EngineSelectionConfig, EngineSelector};
use tempfile::TempDir;

fn bench_engine_selection_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("engine_selection_performance", |b| {
        b.to_async(&rt).iter(|| async {
            let selector = EngineSelector::new();
            let temp_dir = TempDir::new().unwrap();

            // Test different file sizes
            let files = vec![
                ("micro.txt", 1024),    // 1KB - micro file
                ("small.txt", 8192),    // 8KB - small file
                ("medium.txt", 65536),  // 64KB - medium file
                ("large.txt", 1048576), // 1MB - large file
            ];

            for (name, size) in files {
                let source = temp_dir.path().join(name);
                std::fs::write(&source, "A".repeat(size)).unwrap();
                let dest = temp_dir.path().join(format!("dest_{}", name));

                // Benchmark engine selection
                black_box(
                    selector
                        .select_optimal_engine(&source, &dest)
                        .await
                        .unwrap(),
                );
            }
        });
    });
}

fn bench_threshold_adjustment_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("threshold_adjustment_performance", |b| {
        b.to_async(&rt).iter(|| async {
            let mut selector = EngineSelector::new();

            // Simulate performance data collection
            for _ in 0..100 {
                selector
                    .update_performance_history(1024, 1024, 500_000)
                    .await; // Fast micro files
                selector
                    .update_performance_history(8192, 8192, 2_000_000)
                    .await; // Medium small files
                selector
                    .update_performance_history(100_000, 100_000, 10_000_000)
                    .await; // Large files
            }

            // Benchmark threshold adjustment
            black_box(selector.auto_adjust_thresholds().await.unwrap());
        });
    });
}

fn bench_performance_history_update(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("performance_history_update", |b| {
        b.to_async(&rt).iter(|| async {
            let selector = EngineSelector::new();

            // Benchmark performance history updates
            for i in 0..1000 {
                let file_size = 1024 + (i % 100) * 1024; // Varying file sizes
                let copy_time = 500_000 + (i % 50) * 10_000; // Varying copy times
                black_box(
                    selector
                        .update_performance_history(file_size, file_size, copy_time)
                        .await,
                );
            }
        });
    });
}

fn bench_dynamic_vs_static_thresholds(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("threshold_comparison");

    // Benchmark with dynamic thresholds enabled
    group.bench_function("dynamic_thresholds", |b| {
        b.to_async(&rt).iter(|| async {
            let config = EngineSelectionConfig {
                enable_dynamic_thresholds: true,
                min_samples_for_adjustment: 50,
                performance_improvement_threshold: 5.0,
                ..EngineSelectionConfig::default()
            };
            let selector = EngineSelector::with_config(config);
            let temp_dir = TempDir::new().unwrap();

            // Simulate mixed workload
            for i in 0..100 {
                let size = if i % 3 == 0 {
                    2048
                } else if i % 3 == 1 {
                    8192
                } else {
                    65536
                };
                let source = temp_dir.path().join(format!("file_{}.txt", i));
                std::fs::write(&source, "A".repeat(size)).unwrap();
                let dest = temp_dir.path().join(format!("dest_{}.txt", i));

                black_box(
                    selector
                        .select_optimal_engine(&source, &dest)
                        .await
                        .unwrap(),
                );
            }
        });
    });

    // Benchmark with static thresholds
    group.bench_function("static_thresholds", |b| {
        b.to_async(&rt).iter(|| async {
            let config = EngineSelectionConfig {
                enable_dynamic_thresholds: false,
                ..EngineSelectionConfig::default()
            };
            let selector = EngineSelector::with_config(config);
            let temp_dir = TempDir::new().unwrap();

            // Same workload as dynamic test
            for i in 0..100 {
                let size = if i % 3 == 0 {
                    2048
                } else if i % 3 == 1 {
                    8192
                } else {
                    65536
                };
                let source = temp_dir.path().join(format!("file_{}.txt", i));
                std::fs::write(&source, "A".repeat(size)).unwrap();
                let dest = temp_dir.path().join(format!("dest_{}.txt", i));

                black_box(
                    selector
                        .select_optimal_engine(&source, &dest)
                        .await
                        .unwrap(),
                );
            }
        });
    });

    group.finish();
}

fn bench_statistics_collection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("statistics_collection", |b| {
        b.to_async(&rt).iter(|| async {
            let selector = EngineSelector::new();

            // Add some performance data
            for _ in 0..100 {
                selector
                    .update_performance_history(1024, 1024, 1_000_000)
                    .await;
                selector
                    .update_performance_history(8192, 8192, 2_000_000)
                    .await;
                selector
                    .update_performance_history(100_000, 100_000, 10_000_000)
                    .await;
            }

            // Benchmark statistics retrieval
            black_box(selector.get_stats().await);
            black_box(selector.get_performance_summary().await);
            black_box(selector.get_threshold_recommendations().await);
        });
    });
}

fn bench_optimized_thresholds_impact(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("threshold_optimization_impact");

    // Test with old thresholds (1KB micro, 4KB small)
    group.bench_function("old_thresholds", |b| {
        b.to_async(&rt).iter(|| async {
            let config = EngineSelectionConfig {
                micro_file_threshold: 1024, // Old 1KB threshold
                small_file_threshold: 4096, // Old 4KB threshold
                enable_dynamic_thresholds: false,
                ..EngineSelectionConfig::default()
            };
            let selector = EngineSelector::with_config(config);
            let temp_dir = TempDir::new().unwrap();

            // Test files that would benefit from optimized thresholds
            let test_sizes = vec![2048, 3072, 6144, 8192, 12288]; // 2KB, 3KB, 6KB, 8KB, 12KB

            for size in test_sizes {
                let source = temp_dir.path().join(format!("file_{}.txt", size));
                std::fs::write(&source, "A".repeat(size)).unwrap();
                let dest = temp_dir.path().join(format!("dest_{}.txt", size));

                black_box(
                    selector
                        .select_optimal_engine(&source, &dest)
                        .await
                        .unwrap(),
                );
            }
        });
    });

    // Test with new optimized thresholds (4KB micro, 16KB small)
    group.bench_function("optimized_thresholds", |b| {
        b.to_async(&rt).iter(|| async {
            let config = EngineSelectionConfig {
                micro_file_threshold: 4096,  // New 4KB threshold
                small_file_threshold: 16384, // New 16KB threshold
                enable_dynamic_thresholds: false,
                ..EngineSelectionConfig::default()
            };
            let selector = EngineSelector::with_config(config);
            let temp_dir = TempDir::new().unwrap();

            // Same test files
            let test_sizes = vec![2048, 3072, 6144, 8192, 12288]; // 2KB, 3KB, 6KB, 8KB, 12KB

            for size in test_sizes {
                let source = temp_dir.path().join(format!("file_{}.txt", size));
                std::fs::write(&source, "A".repeat(size)).unwrap();
                let dest = temp_dir.path().join(format!("dest_{}.txt", size));

                black_box(
                    selector
                        .select_optimal_engine(&source, &dest)
                        .await
                        .unwrap(),
                );
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_engine_selection_performance,
    bench_threshold_adjustment_performance,
    bench_performance_history_update,
    bench_dynamic_vs_static_thresholds,
    bench_statistics_collection,
    bench_optimized_thresholds_impact
);
criterion_main!(benches);
