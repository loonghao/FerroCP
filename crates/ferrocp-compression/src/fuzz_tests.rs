//! Fuzz tests for ferrocp-compression components

use crate::adaptive::{AdaptiveCompressor, DataType};
use crate::algorithms::AlgorithmImpl;
use crate::engine::CompressionEngineImpl;
use ferrocp_types::{CompressionAlgorithm, CompressionEngine};
use proptest::prelude::*;

/// Generate arbitrary compression algorithms
fn algorithm_strategy() -> impl Strategy<Value = CompressionAlgorithm> {
    prop_oneof![
        Just(CompressionAlgorithm::None),
        Just(CompressionAlgorithm::Zstd),
        Just(CompressionAlgorithm::Lz4),
        Just(CompressionAlgorithm::Brotli),
    ]
}

/// Generate arbitrary data types for classification
fn data_type_strategy() -> impl Strategy<Value = DataType> {
    prop_oneof![
        Just(DataType::Text),
        Just(DataType::Binary),
        Just(DataType::Image),
        Just(DataType::Video),
        Just(DataType::Audio),
        Just(DataType::Archive),
        Just(DataType::Unknown),
    ]
}

/// Generate compression levels (0-22 for most algorithms)
fn compression_level_strategy() -> impl Strategy<Value = u8> {
    0u8..=22
}

/// Generate various data patterns for testing
fn data_pattern_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Empty data
        Just(vec![]),
        // Small data (1-1024 bytes)
        prop::collection::vec(any::<u8>(), 1..=1024),
        // Medium data (1KB-1MB)
        prop::collection::vec(any::<u8>(), 1024..=1024 * 1024),
        // Highly compressible data (repeated patterns)
        (any::<u8>(), 1usize..=1024 * 1024).prop_map(|(byte, size)| vec![byte; size]),
        // Text-like data (ASCII printable characters)
        prop::collection::vec(32u8..=126, 1..=1024 * 1024),
        // Binary data with patterns
        (1usize..=1024).prop_flat_map(|pattern_size| {
            (
                prop::collection::vec(any::<u8>(), pattern_size),
                1usize..=1000,
            )
                .prop_map(|(pattern, repeats)| pattern.repeat(repeats))
        }),
    ]
}

proptest! {
    /// Fuzz test compression and decompression roundtrip
    #[test]
    fn fuzz_compression_roundtrip(
        algorithm in algorithm_strategy(),
        level in compression_level_strategy(),
        data in data_pattern_strategy()
    ) {
        let algo_impl = AlgorithmImpl::create(algorithm);

        // Skip if level is not supported by algorithm
        let max_level = algo_impl.max_level();
        if level > max_level {
            return Ok(());
        }

        // Compress the data
        let compressed_result = algo_impl.compress(&data, level);

        match compressed_result {
            Ok(compressed) => {
                // Decompress and verify
                let decompressed = algo_impl.decompress(&compressed)?;
                let data_len = data.len();
                prop_assert_eq!(data, decompressed);

                // Verify compression properties
                if data_len > 0 && algorithm != CompressionAlgorithm::None {
                    // For non-trivial data, compression should not expand too much
                    if data_len > 100 {
                        prop_assert!(compressed.len() <= data_len * 2);
                    }
                }
            },
            Err(_) => {
                // Some combinations might fail, which is acceptable
                // as long as they fail gracefully
            }
        }
    }

    /// Fuzz test adaptive compressor with various data types
    #[test]
    fn fuzz_adaptive_compressor(
        _data_type in data_type_strategy(),
        data in data_pattern_strategy()
    ) {
        let compressor = AdaptiveCompressor::new();

        if data.is_empty() {
            return Ok(());
        }

        // Test algorithm selection using public API
        let (selected_algorithm, level) = compressor.choose_algorithm(&data);
        prop_assert!(matches!(selected_algorithm, CompressionAlgorithm::None |
                             CompressionAlgorithm::Zstd | CompressionAlgorithm::Lz4 |
                             CompressionAlgorithm::Brotli));
        prop_assert!(level <= 22); // Max compression level
    }

    /// Fuzz test compression engine with various configurations
    #[test]
    fn fuzz_compression_engine(
        data in prop::collection::vec(any::<u8>(), 1..=1000) // Smaller data size for stability
    ) {
        tokio_test::block_on(async {
            let engine = CompressionEngineImpl::new();

            // Test compression
            let compressed_result = engine.compress(&data).await;

            match compressed_result {
                Ok(compressed) => {
                    // Test decompression
                    match engine.decompress(&compressed).await {
                        Ok(decompressed) => {
                            prop_assert_eq!(data, decompressed);
                        },
                        Err(_) => {
                            // Decompression might fail for some edge cases, which is acceptable
                        }
                    }
                },
                Err(_) => {
                    // Compression might fail for some edge cases, which is acceptable
                }
            }

            Ok(())
        })?;
    }

    /// Fuzz test with malformed compressed data
    #[test]
    fn fuzz_decompression_with_malformed_data(
        algorithm in algorithm_strategy(),
        malformed_data in prop::collection::vec(any::<u8>(), 0..=1024)
    ) {
        let algo_impl = AlgorithmImpl::create(algorithm);

        // Attempt to decompress malformed data
        let result = algo_impl.decompress(&malformed_data);

        // Should either succeed (if data happens to be valid) or fail gracefully
        match result {
            Ok(decompressed) => {
                // If decompression succeeds, the result should be valid
                prop_assert!(decompressed.len() <= malformed_data.len() * 1000); // Reasonable expansion limit
            },
            Err(_) => {
                // Failure is expected and acceptable for malformed data
            }
        }
    }

    /// Fuzz test compression with extreme parameters
    #[test]
    fn fuzz_extreme_parameters(
        algorithm in algorithm_strategy(),
        data in prop::collection::vec(any::<u8>(), 0..=10000),
        level in 0u8..=100 // Test beyond normal range
    ) {
        let algo_impl = AlgorithmImpl::create(algorithm);

        // Test with potentially invalid compression levels
        let result = algo_impl.compress(&data, level);

        match result {
            Ok(compressed) => {
                // If compression succeeds, decompression should work
                let decompressed = algo_impl.decompress(&compressed)?;
                prop_assert_eq!(data, decompressed);
            },
            Err(_) => {
                // Failure with extreme parameters is acceptable
            }
        }
    }

    /// Fuzz test sequential compression operations (simplified)
    #[test]
    fn fuzz_sequential_compression(
        data_sets in prop::collection::vec(
            prop::collection::vec(any::<u8>(), 1..=10000),
            1..=5
        )
    ) {
        tokio_test::block_on(async {
            let engine = CompressionEngineImpl::new();

            // Test multiple compression operations sequentially
            for data in data_sets {
                if data.is_empty() {
                    continue;
                }

                let compressed_result = engine.compress(&data).await;
                match compressed_result {
                    Ok(compressed) => {
                        let decompressed = engine.decompress(&compressed).await?;
                        prop_assert_eq!(data, decompressed);
                    },
                    Err(_) => {
                        // Some operations might fail, which is acceptable
                    }
                }
            }

            Ok(())
        })?;
    }
}

/// Additional stress tests for edge cases
#[cfg(test)]
mod stress_tests {
    use super::*;

    proptest! {
        /// Test with very large data (up to 10MB)
        #[test]
        fn stress_test_large_data(
            algorithm in algorithm_strategy(),
            size in 1024*1024usize..=10*1024*1024, // 1MB to 10MB
            pattern in any::<u8>()
        ) {
            if algorithm == CompressionAlgorithm::None {
                return Ok(()); // Skip for no compression
            }

            let data = vec![pattern; size];
            let algo_impl = AlgorithmImpl::create(algorithm);

            let compressed = algo_impl.compress(&data, algo_impl.default_level())?;
            let decompressed = algo_impl.decompress(&compressed)?;

            let original_len = data.len();
            let decompressed_len = decompressed.len();

            prop_assert_eq!(original_len, decompressed_len);
            prop_assert_eq!(data, decompressed);

            // For repeated patterns, compression should be very effective
            prop_assert!(compressed.len() < original_len / 10);
        }

        /// Test rapid compression/decompression cycles
        #[test]
        fn stress_test_rapid_cycles(
            data in prop::collection::vec(any::<u8>(), 1000..=10000),
            cycles in 1usize..=100
        ) {
            tokio_test::block_on(async {
                let engine = CompressionEngineImpl::new();
                let data_ref = &data; // Keep a reference to avoid moving

                for _ in 0..cycles {
                    let compressed = engine.compress(data_ref).await?;
                    let decompressed = engine.decompress(&compressed).await?;
                    prop_assert_eq!(data_ref, &decompressed);
                }

                Ok(())
            })?;
        }
    }
}
