//! Unified test utilities for FerroCP benchmarks and tests
//!
//! This module provides common utilities used across all test files
//! to ensure consistency and reduce code duplication.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test data generation patterns
#[derive(Debug, Clone, Copy)]
pub enum TestDataPattern {
    /// All zeros - highly compressible
    Zeros,
    /// All ones - highly compressible  
    Ones,
    /// Random data - incompressible
    Random,
    /// Mixed compressible/incompressible data
    Mixed,
    /// Realistic file pattern similar to actual files
    Realistic,
}

/// Generate test data with specified pattern
///
/// This is the unified test data generator used across all benchmarks
/// to ensure consistency and reduce code duplication.
pub fn generate_test_data(size: usize, pattern: TestDataPattern) -> Vec<u8> {
    match pattern {
        TestDataPattern::Zeros => vec![0u8; size],
        TestDataPattern::Ones => vec![0xFFu8; size],
        TestDataPattern::Random => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            // Use deterministic "random" data for reproducible benchmarks
            let mut data = Vec::with_capacity(size);
            let mut hasher = DefaultHasher::new();
            
            for i in 0..size {
                i.hash(&mut hasher);
                data.push((hasher.finish() % 256) as u8);
            }
            data
        }
        TestDataPattern::Mixed => {
            // Mix of compressible and incompressible data
            let mut data = Vec::with_capacity(size);
            for i in 0..size {
                if i % 1000 < 100 {
                    data.push(0); // Compressible zeros
                } else if i % 1000 < 200 {
                    data.push(255); // Compressible ones
                } else {
                    data.push((i % 256) as u8); // Semi-random
                }
            }
            data
        }
        TestDataPattern::Realistic => {
            // Realistic file pattern similar to actual files
            let mut data = Vec::with_capacity(size);
            for i in 0..size {
                // Simulate realistic file patterns with some structure
                data.push(((i * 7 + 13) % 256) as u8);
            }
            data
        }
    }
}

/// Create a temporary file with test data
pub fn create_test_file(
    temp_dir: &TempDir, 
    name: &str, 
    size: usize, 
    pattern: TestDataPattern
) -> PathBuf {
    let file_path = temp_dir.path().join(name);
    let data = generate_test_data(size, pattern);
    fs::write(&file_path, data).expect("Failed to write test file");
    file_path
}

/// Create a temporary file with realistic test data (default pattern)
pub fn create_realistic_test_file(temp_dir: &TempDir, name: &str, size: usize) -> PathBuf {
    create_test_file(temp_dir, name, size, TestDataPattern::Realistic)
}

/// Common file sizes for benchmarks
pub struct CommonFileSizes;

impl CommonFileSizes {
    pub const TINY: usize = 1024; // 1KB
    pub const SMALL: usize = 4 * 1024; // 4KB
    pub const MEDIUM: usize = 64 * 1024; // 64KB
    pub const LARGE: usize = 1024 * 1024; // 1MB
    pub const XLARGE: usize = 10 * 1024 * 1024; // 10MB
    pub const XXLARGE: usize = 50 * 1024 * 1024; // 50MB
    pub const HUGE: usize = 100 * 1024 * 1024; // 100MB
    
    /// Get all standard test sizes
    pub fn all() -> Vec<(&'static str, usize)> {
        vec![
            ("1KB", Self::TINY),
            ("4KB", Self::SMALL),
            ("64KB", Self::MEDIUM),
            ("1MB", Self::LARGE),
            ("10MB", Self::XLARGE),
            ("50MB", Self::XXLARGE),
            ("100MB", Self::HUGE),
        ]
    }
    
    /// Get sizes suitable for micro-benchmarks (smaller files)
    pub fn micro() -> Vec<(&'static str, usize)> {
        vec![
            ("1KB", Self::TINY),
            ("4KB", Self::SMALL),
            ("64KB", Self::MEDIUM),
            ("1MB", Self::LARGE),
        ]
    }
    
    /// Get sizes suitable for performance benchmarks (larger files)
    pub fn performance() -> Vec<(&'static str, usize)> {
        vec![
            ("1MB", Self::LARGE),
            ("10MB", Self::XLARGE),
            ("50MB", Self::XXLARGE),
            ("100MB", Self::HUGE),
        ]
    }
}

/// Common buffer sizes for testing
pub struct CommonBufferSizes;

impl CommonBufferSizes {
    pub const SMALL: usize = 4 * 1024; // 4KB
    pub const MEDIUM: usize = 64 * 1024; // 64KB
    pub const LARGE: usize = 256 * 1024; // 256KB
    pub const XLARGE: usize = 512 * 1024; // 512KB (optimal for SSD)
    pub const XXLARGE: usize = 1024 * 1024; // 1MB
    pub const HUGE: usize = 2 * 1024 * 1024; // 2MB
    
    /// Get all standard buffer sizes
    pub fn all() -> Vec<(&'static str, usize)> {
        vec![
            ("4KB", Self::SMALL),
            ("64KB", Self::MEDIUM),
            ("256KB", Self::LARGE),
            ("512KB", Self::XLARGE),
            ("1MB", Self::XXLARGE),
            ("2MB", Self::HUGE),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_test_data_patterns() {
        let size = 1024;
        
        // Test zeros pattern
        let zeros = generate_test_data(size, TestDataPattern::Zeros);
        assert_eq!(zeros.len(), size);
        assert!(zeros.iter().all(|&b| b == 0));
        
        // Test ones pattern
        let ones = generate_test_data(size, TestDataPattern::Ones);
        assert_eq!(ones.len(), size);
        assert!(ones.iter().all(|&b| b == 0xFF));
        
        // Test realistic pattern
        let realistic = generate_test_data(size, TestDataPattern::Realistic);
        assert_eq!(realistic.len(), size);
        
        // Test mixed pattern
        let mixed = generate_test_data(size, TestDataPattern::Mixed);
        assert_eq!(mixed.len(), size);
    }
    
    #[test]
    fn test_create_test_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "test.dat", 1024, TestDataPattern::Zeros);
        
        assert!(file_path.exists());
        assert_eq!(fs::metadata(&file_path).unwrap().len(), 1024);
    }
}
