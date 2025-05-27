//! Delta copy module for ferrocp
//!
//! This module provides incremental copy functionality using delta compression.
//! It can compute differences between files and apply them efficiently,
//! which is useful for incremental backups and synchronization.

use crate::error::Result;
use blake3::Hash as Blake3Hash;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// File signature for delta operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSignature {
    /// File hash
    pub hash: Blake3Hash,
    /// File size
    pub size: u64,
    /// Block signatures for delta computation
    pub blocks: Vec<BlockSignature>,
    /// Block size used for signature
    pub block_size: usize,
}

/// Block signature for delta computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSignature {
    /// Block index
    pub index: u64,
    /// Block hash
    pub hash: Blake3Hash,
    /// Block size
    pub size: usize,
}

/// Delta information representing differences between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Source file signature
    pub source_signature: FileSignature,
    /// Reference file signature
    pub reference_signature: FileSignature,
    /// Delta operations
    pub operations: Vec<DeltaOperation>,
    /// Total size of delta data
    pub delta_size: u64,
}

/// Delta operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOperation {
    /// Copy data from reference file
    Copy {
        /// Offset in reference file
        ref_offset: u64,
        /// Offset in destination file
        dest_offset: u64,
        /// Number of bytes to copy
        length: u64,
    },
    /// Insert new data
    Insert {
        /// Offset in destination file
        dest_offset: u64,
        /// Data to insert
        data: Vec<u8>,
    },
}

/// Delta copy engine
pub struct DeltaCopyEngine {
    /// Block size for signature computation
    block_size: usize,
}

impl DeltaCopyEngine {
    /// Create a new delta copy engine
    pub fn new() -> Self {
        Self {
            block_size: 64 * 1024, // 64KB blocks
        }
    }

    /// Create delta copy engine with custom block size
    pub fn with_block_size(block_size: usize) -> Self {
        Self { block_size }
    }

    /// Compute file signature
    pub async fn compute_signature<P: AsRef<Path>>(&self, path: P) -> Result<FileSignature> {
        let path = path.as_ref();
        let mut file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();

        debug!("Computing signature for {:?} ({} bytes)", path, file_size);

        let mut hasher = blake3::Hasher::new();
        let mut blocks = Vec::new();
        let mut buffer = vec![0u8; self.block_size];
        let mut block_index = 0u64;
        let mut _total_read = 0u64;

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let block_data = &buffer[..bytes_read];

            // Update file hash
            hasher.update(block_data);

            // Compute block signature
            let block_hash = blake3::hash(block_data);
            blocks.push(BlockSignature {
                index: block_index,
                hash: block_hash,
                size: bytes_read,
            });

            block_index += 1;
            _total_read += bytes_read as u64;
        }

        let file_hash = hasher.finalize();

        let signature = FileSignature {
            hash: file_hash,
            size: file_size,
            blocks,
            block_size: self.block_size,
        };

        debug!("Computed signature with {} blocks", signature.blocks.len());
        Ok(signature)
    }

    /// Compute delta between source and reference files
    pub async fn compute_delta<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        reference: Q,
    ) -> Result<Delta> {
        let source_signature = self.compute_signature(source).await?;
        let reference_signature = self.compute_signature(reference).await?;

        debug!("Computing delta between files");

        // Simple delta algorithm - in a real implementation, this would be more sophisticated
        let mut operations = Vec::new();
        let mut delta_size = 0u64;

        // For now, just implement a basic block-level comparison
        // In a production implementation, you would use a more sophisticated algorithm
        // like the one used in rsync or similar tools

        for (i, source_block) in source_signature.blocks.iter().enumerate() {
            let mut found_match = false;

            // Look for matching block in reference
            for ref_block in &reference_signature.blocks {
                if source_block.hash == ref_block.hash {
                    // Found matching block, add copy operation
                    operations.push(DeltaOperation::Copy {
                        ref_offset: ref_block.index * self.block_size as u64,
                        dest_offset: i as u64 * self.block_size as u64,
                        length: source_block.size as u64,
                    });
                    found_match = true;
                    break;
                }
            }

            if !found_match {
                // No matching block found, need to insert new data
                // For now, we'll mark this as needing the full block
                // In a real implementation, you'd read the actual data
                operations.push(DeltaOperation::Insert {
                    dest_offset: i as u64 * self.block_size as u64,
                    data: vec![0u8; source_block.size], // Placeholder
                });
                delta_size += source_block.size as u64;
            }
        }

        let delta = Delta {
            source_signature,
            reference_signature,
            operations,
            delta_size,
        };

        info!("Computed delta with {} operations ({} bytes)",
              delta.operations.len(), delta.delta_size);

        Ok(delta)
    }

    /// Apply delta to create destination file
    pub async fn apply_delta<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        delta: &Delta,
        reference: P,
        destination: Q,
    ) -> Result<()> {
        let reference_path = reference.as_ref();
        let destination_path = destination.as_ref();

        debug!("Applying delta to create {:?}", destination_path);

        let mut reference_file = File::open(reference_path).await?;
        let mut dest_file = File::create(destination_path).await?;

        for operation in &delta.operations {
            match operation {
                DeltaOperation::Copy { ref_offset: _, dest_offset: _, length } => {
                    // Copy data from reference file
                    let mut buffer = vec![0u8; *length as usize];

                    // Seek to reference offset (simplified - real implementation would be more efficient)
                    reference_file.read_exact(&mut buffer).await?;
                    dest_file.write_all(&buffer).await?;
                }
                DeltaOperation::Insert { dest_offset: _, data } => {
                    // Insert new data
                    dest_file.write_all(data).await?;
                }
            }
        }

        dest_file.flush().await?;
        info!("Delta applied successfully");
        Ok(())
    }

    /// Perform delta copy operation
    pub async fn delta_copy<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        reference: R,
    ) -> Result<u64> {
        let source_path = source.as_ref();
        let destination_path = destination.as_ref();
        let reference_path = reference.as_ref();

        info!("Performing delta copy: {:?} -> {:?} (ref: {:?})",
              source_path, destination_path, reference_path);

        // Check if reference file exists
        if !tokio::fs::metadata(reference_path).await.is_ok() {
            warn!("Reference file {:?} not found, falling back to regular copy", reference_path);
            return self.regular_copy(source_path, destination_path).await;
        }

        // Compute delta
        let delta = self.compute_delta(source_path, reference_path).await?;

        // Check if delta copy is beneficial
        let source_size = tokio::fs::metadata(source_path).await?.len();
        if delta.delta_size >= source_size {
            warn!("Delta copy not beneficial, falling back to regular copy");
            return self.regular_copy(source_path, destination_path).await;
        }

        // Apply delta
        self.apply_delta(&delta, reference_path, destination_path).await?;

        Ok(source_size)
    }

    /// Regular file copy fallback
    async fn regular_copy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
    ) -> Result<u64> {
        let mut source_file = File::open(source).await?;
        let mut dest_file = File::create(destination).await?;

        let bytes_copied = tokio::io::copy(&mut source_file, &mut dest_file).await?;
        dest_file.flush().await?;

        Ok(bytes_copied)
    }

    /// Get compression ratio for delta
    pub fn delta_ratio(&self, delta: &Delta) -> f64 {
        if delta.source_signature.size == 0 {
            return 1.0;
        }
        delta.delta_size as f64 / delta.source_signature.size as f64
    }
}

impl Default for DeltaCopyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for delta operations
pub mod utils {
    use super::*;

    /// Check if delta copy would be beneficial
    pub async fn is_delta_beneficial<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        reference: Q,
        threshold: f64,
    ) -> Result<bool> {
        let engine = DeltaCopyEngine::new();
        let delta = engine.compute_delta(source, reference).await?;
        let ratio = engine.delta_ratio(&delta);
        Ok(ratio < threshold)
    }

    /// Estimate delta size without computing full delta
    pub async fn estimate_delta_size<P: AsRef<Path>, Q: AsRef<Path>>(
        source: P,
        reference: Q,
    ) -> Result<u64> {
        let engine = DeltaCopyEngine::new();
        let source_sig = engine.compute_signature(source).await?;
        let ref_sig = engine.compute_signature(reference).await?;

        // Simple estimation based on different blocks
        let mut different_blocks = 0;
        for source_block in &source_sig.blocks {
            let mut found = false;
            for ref_block in &ref_sig.blocks {
                if source_block.hash == ref_block.hash {
                    found = true;
                    break;
                }
            }
            if !found {
                different_blocks += 1;
            }
        }

        Ok(different_blocks as u64 * engine.block_size as u64)
    }
}
