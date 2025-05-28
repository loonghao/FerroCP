//! Delta compression for efficient file synchronization

use ferrocp_types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Delta patch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOperation {
    /// Copy bytes from source file
    Copy {
        /// Offset in source file
        offset: u64,
        /// Number of bytes to copy
        length: u64,
    },
    /// Insert new data
    Insert {
        /// Data to insert
        data: Vec<u8>,
    },
    /// Delete bytes
    Delete {
        /// Number of bytes to delete
        length: u64,
    },
}

/// Delta patch containing operations to transform one file to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPatch {
    /// Source file hash
    pub source_hash: String,
    /// Target file hash
    pub target_hash: String,
    /// List of operations
    pub operations: Vec<DeltaOperation>,
    /// Compressed size of the patch
    pub compressed_size: usize,
    /// Uncompressed size of the patch
    pub uncompressed_size: usize,
}

impl DeltaPatch {
    /// Create a new delta patch
    pub fn new(source_hash: String, target_hash: String) -> Self {
        Self {
            source_hash,
            target_hash,
            operations: Vec::new(),
            compressed_size: 0,
            uncompressed_size: 0,
        }
    }

    /// Add an operation to the patch
    pub fn add_operation(&mut self, operation: DeltaOperation) {
        self.operations.push(operation);
    }

    /// Get the compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size > 0 {
            self.compressed_size as f64 / self.uncompressed_size as f64
        } else {
            1.0
        }
    }

    /// Check if the patch is worth applying (good compression ratio)
    pub fn is_efficient(&self, threshold: f64) -> bool {
        self.compression_ratio() < threshold
    }

    /// Serialize the patch to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| Error::Network {
            message: format!("Failed to serialize delta patch: {}", e),
        })
    }

    /// Deserialize patch from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| Error::Network {
            message: format!("Failed to deserialize delta patch: {}", e),
        })
    }
}

/// Configuration for delta compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaConfig {
    /// Block size for rolling hash
    pub block_size: usize,
    /// Window size for similarity detection
    pub window_size: usize,
    /// Minimum match length
    pub min_match_length: usize,
    /// Maximum file size for delta compression
    pub max_file_size: u64,
    /// Compression efficiency threshold
    pub efficiency_threshold: f64,
}

impl Default for DeltaConfig {
    fn default() -> Self {
        Self {
            block_size: 4096,
            window_size: 16,
            min_match_length: 32,
            max_file_size: 100 * 1024 * 1024, // 100MB
            efficiency_threshold: 0.8,        // Only use delta if it's at least 20% smaller
        }
    }
}

/// Engine for creating and applying delta patches
#[derive(Debug)]
pub struct DeltaEngine {
    config: DeltaConfig,
}

impl DeltaEngine {
    /// Create a new delta engine
    pub fn new(config: DeltaConfig) -> Self {
        Self { config }
    }

    /// Create a delta engine with default configuration
    pub fn default() -> Self {
        Self::new(DeltaConfig::default())
    }

    /// Create a delta patch between two files
    pub async fn create_patch<P: AsRef<Path>>(
        &self,
        source_path: P,
        target_path: P,
    ) -> Result<Option<DeltaPatch>> {
        let source_path = source_path.as_ref();
        let target_path = target_path.as_ref();

        // Read both files
        let source_data = fs::read(source_path).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to read source file '{}': {}",
                source_path.display(),
                e
            ),
        })?;

        let target_data = fs::read(target_path).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to read target file '{}': {}",
                target_path.display(),
                e
            ),
        })?;

        self.create_patch_from_data(&source_data, &target_data)
            .await
    }

    /// Create a delta patch from data
    pub async fn create_patch_from_data(
        &self,
        source_data: &[u8],
        target_data: &[u8],
    ) -> Result<Option<DeltaPatch>> {
        // Check file size limits
        if source_data.len() as u64 > self.config.max_file_size
            || target_data.len() as u64 > self.config.max_file_size
        {
            debug!("Files too large for delta compression");
            return Ok(None);
        }

        // Calculate hashes
        let source_hash = blake3::hash(source_data).to_hex().to_string();
        let target_hash = blake3::hash(target_data).to_hex().to_string();

        // If files are identical, no patch needed
        if source_hash == target_hash {
            debug!("Files are identical, no patch needed");
            return Ok(None);
        }

        let mut patch = DeltaPatch::new(source_hash, target_hash);

        // Simple delta algorithm - this is a basic implementation
        // In a real-world scenario, you'd use more sophisticated algorithms like rsync's
        let operations = self.compute_delta_operations(source_data, target_data);

        for operation in operations {
            patch.add_operation(operation);
        }

        // Calculate patch size
        let patch_bytes = patch.to_bytes()?;
        patch.compressed_size = patch_bytes.len();
        patch.uncompressed_size = target_data.len();

        // Check if patch is efficient
        if !patch.is_efficient(self.config.efficiency_threshold) {
            debug!(
                "Delta patch not efficient: ratio = {:.2}, threshold = {:.2}, compressed_size = {}, uncompressed_size = {}",
                patch.compression_ratio(),
                self.config.efficiency_threshold,
                patch.compressed_size,
                patch.uncompressed_size
            );
            return Ok(None);
        }

        info!(
            "Created delta patch: {} -> {} bytes (ratio: {:.2})",
            patch.uncompressed_size,
            patch.compressed_size,
            patch.compression_ratio()
        );

        Ok(Some(patch))
    }

    /// Apply a delta patch to create the target file
    pub async fn apply_patch<P: AsRef<Path>>(
        &self,
        source_path: P,
        patch: &DeltaPatch,
        output_path: P,
    ) -> Result<()> {
        let source_path = source_path.as_ref();
        let output_path = output_path.as_ref();

        // Read source file
        let source_data = fs::read(source_path).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to read source file '{}': {}",
                source_path.display(),
                e
            ),
        })?;

        // Verify source hash
        let source_hash = blake3::hash(&source_data).to_hex().to_string();
        if source_hash != patch.source_hash {
            return Err(Error::Sync {
                message: "Source file hash mismatch".to_string(),
            });
        }

        // Apply patch operations
        let target_data = self.apply_operations(&source_data, &patch.operations)?;

        // Verify target hash
        let target_hash = blake3::hash(&target_data).to_hex().to_string();
        if target_hash != patch.target_hash {
            return Err(Error::Sync {
                message: "Target file hash mismatch after applying patch".to_string(),
            });
        }

        // Write target file
        fs::write(output_path, target_data)
            .await
            .map_err(|e| Error::Io {
                message: format!(
                    "Failed to write output file '{}': {}",
                    output_path.display(),
                    e
                ),
            })?;

        info!("Applied delta patch successfully");
        Ok(())
    }

    /// Compute delta operations (simplified algorithm)
    fn compute_delta_operations(&self, source: &[u8], target: &[u8]) -> Vec<DeltaOperation> {
        let mut operations = Vec::new();
        let mut target_pos = 0;

        while target_pos < target.len() {
            // Look for matches in source
            let remaining_target = &target[target_pos..];
            let match_result = self.find_best_match(source, remaining_target);

            match match_result {
                Some((source_offset, match_length))
                    if match_length >= self.config.min_match_length =>
                {
                    // Found a good match, add copy operation
                    operations.push(DeltaOperation::Copy {
                        offset: source_offset as u64,
                        length: match_length as u64,
                    });
                    target_pos += match_length;
                }
                _ => {
                    // No good match found, collect bytes to insert
                    let mut insert_data = Vec::new();
                    let start_pos = target_pos;

                    // Collect consecutive bytes that don't have good matches
                    while target_pos < target.len() {
                        let remaining = &target[target_pos..];
                        if let Some((_, match_len)) = self.find_best_match(source, remaining) {
                            if match_len >= self.config.min_match_length {
                                break;
                            }
                        }
                        target_pos += 1;
                    }

                    insert_data.extend_from_slice(&target[start_pos..target_pos]);
                    if !insert_data.is_empty() {
                        operations.push(DeltaOperation::Insert { data: insert_data });
                    }
                }
            }
        }

        operations
    }

    /// Find the best match for a target slice in the source data
    fn find_best_match(&self, source: &[u8], target: &[u8]) -> Option<(usize, usize)> {
        if target.is_empty() {
            return None;
        }

        let mut best_match = None;
        let mut best_length = 0;

        // Simple brute force search - in practice, you'd use rolling hash
        for source_pos in 0..source.len() {
            let remaining_source = &source[source_pos..];
            let match_length = self.common_prefix_length(remaining_source, target);

            if match_length > best_length {
                best_length = match_length;
                best_match = Some((source_pos, match_length));
            }
        }

        best_match
    }

    /// Calculate the length of common prefix between two slices
    fn common_prefix_length(&self, a: &[u8], b: &[u8]) -> usize {
        let max_len = a.len().min(b.len());
        for i in 0..max_len {
            if a[i] != b[i] {
                return i;
            }
        }
        max_len
    }

    /// Apply delta operations to source data
    fn apply_operations(&self, source: &[u8], operations: &[DeltaOperation]) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        for operation in operations {
            match operation {
                DeltaOperation::Copy { offset, length } => {
                    let start = *offset as usize;
                    let end = start + *length as usize;

                    if end > source.len() {
                        return Err(Error::Sync {
                            message: "Copy operation exceeds source file bounds".to_string(),
                        });
                    }

                    result.extend_from_slice(&source[start..end]);
                }
                DeltaOperation::Insert { data } => {
                    result.extend_from_slice(data);
                }
                DeltaOperation::Delete { length: _ } => {
                    // Delete operations are implicit in our model
                    // (we only copy/insert what we need)
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_delta_patch_creation() {
        let source_data = b"Hello, World! This is a test file.";
        let target_data = b"Hello, Universe! This is a test file with changes.";

        // Use a more lenient efficiency threshold for testing
        let config = DeltaConfig {
            efficiency_threshold: 10.0, // Allow patches up to 10x the target size for small test files
            min_match_length: 4,        // Lower minimum match length for small test data
            ..Default::default()
        };
        let engine = DeltaEngine::new(config);
        let patch = engine
            .create_patch_from_data(source_data, target_data)
            .await
            .unwrap();

        assert!(
            patch.is_some(),
            "Patch should be created for different files"
        );
        let patch = patch.unwrap();
        assert!(!patch.operations.is_empty());
    }

    #[tokio::test]
    async fn test_delta_patch_application() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let target_path = temp_dir.path().join("target.txt");
        let output_path = temp_dir.path().join("output.txt");

        let source_data = b"Hello, World! This is a test file.";
        let target_data = b"Hello, Universe! This is a test file with changes.";

        fs::write(&source_path, source_data).await.unwrap();
        fs::write(&target_path, target_data).await.unwrap();

        // Use a more lenient efficiency threshold for testing
        let config = DeltaConfig {
            efficiency_threshold: 10.0, // Allow patches up to 10x the target size for small test files
            min_match_length: 4,        // Lower minimum match length for small test data
            ..Default::default()
        };
        let engine = DeltaEngine::new(config);

        // Create patch
        let patch = engine
            .create_patch(&source_path, &target_path)
            .await
            .unwrap();
        assert!(patch.is_some());
        let patch = patch.unwrap();

        // Apply patch
        engine
            .apply_patch(&source_path, &patch, &output_path)
            .await
            .unwrap();

        // Verify result
        let output_data = fs::read(&output_path).await.unwrap();
        assert_eq!(output_data, target_data);
    }

    #[test]
    fn test_delta_operation_serialization() {
        let operation = DeltaOperation::Copy {
            offset: 100,
            length: 50,
        };

        let mut patch = DeltaPatch::new("source_hash".to_string(), "target_hash".to_string());
        patch.add_operation(operation);

        let bytes = patch.to_bytes().unwrap();
        let deserialized = DeltaPatch::from_bytes(&bytes).unwrap();

        assert_eq!(patch.source_hash, deserialized.source_hash);
        assert_eq!(patch.target_hash, deserialized.target_hash);
        assert_eq!(patch.operations.len(), deserialized.operations.len());
    }
}
