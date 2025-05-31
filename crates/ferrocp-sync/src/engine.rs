//! Main synchronization engine

use crate::{
    cache::{CacheConfig, HashCache},
    conflict::{ConflictConfig, ConflictFileInfo, ConflictResolver, ConflictType, FileConflict},
    delta::{DeltaConfig, DeltaEngine},
    diff::{ChangeType, DiffConfig, DiffEngine},
    progress::{ProgressReporter, SyncPhase},
};
use ferrocp_types::{CopyStats, Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::{debug, info, warn};

/// Synchronization request
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Source directory path
    pub source: PathBuf,
    /// Destination directory path
    pub destination: PathBuf,
    /// Sync options
    pub options: SyncOptions,
    /// Request ID for tracking
    pub request_id: uuid::Uuid,
}

impl SyncRequest {
    /// Create a new sync request
    pub fn new<P: AsRef<Path>>(source: P, destination: P) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            options: SyncOptions::default(),
            request_id: uuid::Uuid::new_v4(),
        }
    }

    /// Set sync options
    pub fn with_options(mut self, options: SyncOptions) -> Self {
        self.options = options;
        self
    }
}

/// Synchronization options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOptions {
    /// Enable incremental synchronization
    pub incremental: bool,
    /// Enable delta compression
    pub enable_delta: bool,
    /// Enable hash caching
    pub enable_caching: bool,
    /// Delete files in destination that don't exist in source
    pub delete_extra: bool,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Preserve file permissions
    pub preserve_permissions: bool,
    /// Preserve file timestamps
    pub preserve_timestamps: bool,
    /// Dry run (don't actually modify files)
    pub dry_run: bool,
    /// Maximum file size for delta compression
    pub max_delta_size: u64,
    /// Conflict resolution configuration
    pub conflict_config: ConflictConfig,
    /// Diff detection configuration
    pub diff_config: DiffConfig,
    /// Delta compression configuration
    pub delta_config: DeltaConfig,
    /// Cache configuration
    pub cache_config: CacheConfig,
}

impl SyncOptions {
    /// Create options for incremental sync
    pub fn incremental() -> Self {
        Self {
            incremental: true,
            enable_delta: true,
            enable_caching: true,
            ..Default::default()
        }
    }

    /// Create options for full sync
    pub fn full() -> Self {
        Self {
            incremental: false,
            enable_delta: false,
            enable_caching: false,
            ..Default::default()
        }
    }

    /// Create options for mirror sync (delete extra files)
    pub fn mirror() -> Self {
        Self {
            delete_extra: true,
            ..Self::incremental()
        }
    }
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            incremental: true,
            enable_delta: true,
            enable_caching: true,
            delete_extra: false,
            follow_symlinks: false,
            preserve_permissions: true,
            preserve_timestamps: true,
            dry_run: false,
            max_delta_size: 100 * 1024 * 1024, // 100MB
            conflict_config: ConflictConfig::default(),
            diff_config: DiffConfig::default(),
            delta_config: DeltaConfig::default(),
            cache_config: CacheConfig::default(),
        }
    }
}

/// Synchronization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Request ID
    pub request_id: uuid::Uuid,
    /// Copy statistics
    pub stats: CopyStats,
    /// Number of files synced
    pub files_synced: u64,
    /// Number of bytes transferred
    pub bytes_transferred: u64,
    /// Number of conflicts encountered
    pub conflicts_count: u64,
    /// Number of errors encountered
    pub errors_count: u64,
    /// Sync duration
    pub duration: Duration,
    /// Whether delta compression was used
    pub delta_used: bool,
    /// Delta compression savings (bytes)
    pub delta_savings: u64,
}

impl SyncResult {
    /// Create a new sync result
    pub fn new(request_id: uuid::Uuid, stats: CopyStats) -> Self {
        Self {
            request_id,
            stats,
            files_synced: 0,
            bytes_transferred: 0,
            conflicts_count: 0,
            errors_count: 0,
            duration: Duration::default(),
            delta_used: false,
            delta_savings: 0,
        }
    }
}

/// Main synchronization engine
#[derive(Debug)]
pub struct SyncEngine {
    diff_engine: DiffEngine,
    #[allow(dead_code)]
    delta_engine: DeltaEngine,
    hash_cache: Option<HashCache>,
    conflict_resolver: ConflictResolver,
}

impl SyncEngine {
    /// Create a new sync engine
    pub async fn new() -> Result<Self> {
        Self::with_options(SyncOptions::default()).await
    }

    /// Create a sync engine with custom options
    pub async fn with_options(options: SyncOptions) -> Result<Self> {
        let diff_engine = DiffEngine::new(options.diff_config.clone());
        let delta_engine = DeltaEngine::new(options.delta_config.clone());
        let conflict_resolver = ConflictResolver::new(options.conflict_config.clone());

        let hash_cache = if options.enable_caching {
            Some(HashCache::new(options.cache_config.clone()).await?)
        } else {
            None
        };

        Ok(Self {
            diff_engine,
            delta_engine,
            hash_cache,
            conflict_resolver,
        })
    }

    /// Perform synchronization
    pub async fn sync(&mut self, request: SyncRequest) -> Result<SyncResult> {
        let start_time = Instant::now();
        let progress_reporter = ProgressReporter::new(request.request_id);

        info!(
            "Starting sync: {} -> {}",
            request.source.display(),
            request.destination.display()
        );

        // Initialize result
        let mut result = SyncResult::new(request.request_id, CopyStats::new());

        // Phase 1: Initialize
        progress_reporter.set_phase(SyncPhase::Initializing).await;
        self.validate_paths(&request.source, &request.destination)
            .await?;

        // Phase 2: Scan source directory
        progress_reporter.set_phase(SyncPhase::ScanningSource).await;
        let source_files = self.diff_engine.scan_directory(&request.source).await?;
        info!("Found {} files in source", source_files.len());

        // Phase 3: Scan destination directory
        progress_reporter
            .set_phase(SyncPhase::ScanningDestination)
            .await;
        let dest_files = if request.destination.exists() {
            self.diff_engine
                .scan_directory(&request.destination)
                .await?
        } else {
            std::collections::HashMap::new()
        };
        info!("Found {} files in destination", dest_files.len());

        // Set totals for progress tracking
        let total_bytes: u64 = source_files.values().map(|f| f.size).sum();
        progress_reporter
            .set_totals(source_files.len() as u64, total_bytes)
            .await;

        // Phase 4: Detect changes
        progress_reporter
            .set_phase(SyncPhase::DetectingChanges)
            .await;
        let changes = self.diff_engine.detect_changes(&source_files, &dest_files);
        info!("Detected {} changes", changes.len());

        // Phase 5: Resolve conflicts
        progress_reporter
            .set_phase(SyncPhase::ResolvingConflicts)
            .await;
        self.detect_and_resolve_conflicts(&changes, &source_files, &dest_files)
            .await?;
        result.conflicts_count = self.conflict_resolver.get_stats().total_conflicts as u64;

        // Phase 6: Transfer files
        progress_reporter.set_phase(SyncPhase::Transferring).await;
        let transfer_result = self
            .transfer_files(&request, &changes, &progress_reporter)
            .await?;
        result.files_synced = transfer_result.files_copied;
        result.bytes_transferred = transfer_result.bytes_copied;

        // Phase 7: Apply deltas (if enabled)
        if request.options.enable_delta {
            progress_reporter.set_phase(SyncPhase::ApplyingDeltas).await;
            // TODO: Implement delta application
        }

        // Phase 8: Finalize
        progress_reporter.set_phase(SyncPhase::Finalizing).await;
        if let Some(cache) = &mut self.hash_cache {
            cache.save().await?;
        }

        // Complete
        result.duration = start_time.elapsed();
        progress_reporter.completed().await;

        info!(
            "Sync completed: {} files, {} bytes in {:?}",
            result.files_synced, result.bytes_transferred, result.duration
        );

        Ok(result)
    }

    /// Validate source and destination paths
    async fn validate_paths(&self, source: &Path, destination: &Path) -> Result<()> {
        if !source.exists() {
            return Err(Error::Io {
                message: format!("Source path does not exist: {}", source.display()),
            });
        }

        if !source.is_dir() {
            return Err(Error::Io {
                message: format!("Source path is not a directory: {}", source.display()),
            });
        }

        // Create destination directory if it doesn't exist
        if !destination.exists() {
            fs::create_dir_all(destination)
                .await
                .map_err(|e| Error::Io {
                    message: format!(
                        "Failed to create destination directory '{}': {}",
                        destination.display(),
                        e
                    ),
                })?;
        }

        Ok(())
    }

    /// Detect and resolve conflicts
    async fn detect_and_resolve_conflicts(
        &mut self,
        changes: &[crate::diff::FileChange],
        source_files: &std::collections::HashMap<PathBuf, crate::diff::FileInfo>,
        dest_files: &std::collections::HashMap<PathBuf, crate::diff::FileInfo>,
    ) -> Result<()> {
        for change in changes {
            if change.change_type == ChangeType::Modified {
                // Check if this is a real conflict
                if let (Some(source_info), Some(dest_info)) =
                    (source_files.get(&change.path), dest_files.get(&change.path))
                {
                    // Create conflict if both files were modified
                    let conflict =
                        FileConflict::new(change.path.clone(), ConflictType::BothModified)
                            .with_source_info(ConflictFileInfo {
                                size: source_info.size,
                                modified: source_info.modified,
                                hash: source_info.hash.clone(),
                                is_directory: source_info.is_directory,
                                permissions: source_info.permissions,
                            })
                            .with_destination_info(ConflictFileInfo {
                                size: dest_info.size,
                                modified: dest_info.modified,
                                hash: dest_info.hash.clone(),
                                is_directory: dest_info.is_directory,
                                permissions: dest_info.permissions,
                            });

                    self.conflict_resolver.add_conflict(conflict);
                }
            }
        }

        // Auto-resolve conflicts if enabled
        let resolved_count = self.conflict_resolver.auto_resolve_conflicts();
        if resolved_count > 0 {
            info!("Auto-resolved {} conflicts", resolved_count);
        }

        // Check for unresolved conflicts
        let unresolved = self.conflict_resolver.get_unresolved_conflicts();
        if !unresolved.is_empty() {
            warn!("Found {} unresolved conflicts", unresolved.len());
            // TODO: Handle unresolved conflicts (ask user, fail, etc.)
        }

        Ok(())
    }

    /// Transfer files based on detected changes
    async fn transfer_files(
        &mut self,
        request: &SyncRequest,
        changes: &[crate::diff::FileChange],
        progress_reporter: &ProgressReporter,
    ) -> Result<CopyStats> {
        let mut stats = CopyStats::new();

        for change in changes {
            let source_path = request.source.join(&change.path);
            let dest_path = request.destination.join(&change.path);

            progress_reporter.file_started(change.path.clone()).await;

            match change.change_type {
                ChangeType::Added | ChangeType::Modified => {
                    if request.options.dry_run {
                        debug!(
                            "DRY RUN: Would copy {} -> {}",
                            source_path.display(),
                            dest_path.display()
                        );
                    } else {
                        self.copy_file(&source_path, &dest_path, &request.options)
                            .await?;
                    }
                    stats.files_copied += 1;
                    stats.bytes_copied += change.size;
                }
                ChangeType::Deleted => {
                    if request.options.delete_extra {
                        if request.options.dry_run {
                            debug!("DRY RUN: Would delete {}", dest_path.display());
                        } else {
                            self.delete_file(&dest_path).await?;
                        }
                    }
                }
                ChangeType::Moved => {
                    if let Some(old_path) = &change.old_path {
                        let old_dest_path = request.destination.join(old_path);
                        if request.options.dry_run {
                            debug!(
                                "DRY RUN: Would move {} -> {}",
                                old_dest_path.display(),
                                dest_path.display()
                            );
                        } else {
                            self.move_file(&old_dest_path, &dest_path).await?;
                        }
                    }
                }
                ChangeType::MetadataChanged => {
                    if request.options.dry_run {
                        debug!("DRY RUN: Would update metadata for {}", dest_path.display());
                    } else {
                        self.update_metadata(&source_path, &dest_path, &request.options)
                            .await?;
                    }
                }
            }

            progress_reporter
                .file_completed(change.path.clone(), change.size)
                .await;
        }

        Ok(stats)
    }

    /// Copy a single file
    async fn copy_file(
        &mut self,
        source: &Path,
        destination: &Path,
        options: &SyncOptions,
    ) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await.map_err(|e| Error::Io {
                message: format!("Failed to create directory '{}': {}", parent.display(), e),
            })?;
        }

        // Copy file content
        fs::copy(source, destination).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to copy '{}' to '{}': {}",
                source.display(),
                destination.display(),
                e
            ),
        })?;

        // Preserve timestamps if requested
        if options.preserve_timestamps {
            let metadata = fs::metadata(source).await.map_err(|e| Error::Io {
                message: format!("Failed to get metadata for '{}': {}", source.display(), e),
            })?;

            if let Ok(modified) = metadata.modified() {
                filetime::set_file_mtime(
                    destination,
                    filetime::FileTime::from_system_time(modified),
                )
                .map_err(|e| Error::Io {
                    message: format!(
                        "Failed to set modification time for '{}': {}",
                        destination.display(),
                        e
                    ),
                })?;
            }
        }

        debug!("Copied: {} -> {}", source.display(), destination.display());
        Ok(())
    }

    /// Delete a file or directory
    async fn delete_file(&self, path: &Path) -> Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path).await.map_err(|e| Error::Io {
                message: format!("Failed to delete directory '{}': {}", path.display(), e),
            })?;
        } else {
            fs::remove_file(path).await.map_err(|e| Error::Io {
                message: format!("Failed to delete file '{}': {}", path.display(), e),
            })?;
        }

        debug!("Deleted: {}", path.display());
        Ok(())
    }

    /// Move a file
    async fn move_file(&self, source: &Path, destination: &Path) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await.map_err(|e| Error::Io {
                message: format!("Failed to create directory '{}': {}", parent.display(), e),
            })?;
        }

        fs::rename(source, destination)
            .await
            .map_err(|e| Error::Io {
                message: format!(
                    "Failed to move '{}' to '{}': {}",
                    source.display(),
                    destination.display(),
                    e
                ),
            })?;

        debug!("Moved: {} -> {}", source.display(), destination.display());
        Ok(())
    }

    /// Update file metadata
    async fn update_metadata(
        &self,
        source: &Path,
        destination: &Path,
        options: &SyncOptions,
    ) -> Result<()> {
        if options.preserve_timestamps {
            let metadata = fs::metadata(source).await.map_err(|e| Error::Io {
                message: format!("Failed to get metadata for '{}': {}", source.display(), e),
            })?;

            if let Ok(modified) = metadata.modified() {
                filetime::set_file_mtime(
                    destination,
                    filetime::FileTime::from_system_time(modified),
                )
                .map_err(|e| Error::Io {
                    message: format!(
                        "Failed to set modification time for '{}': {}",
                        destination.display(),
                        e
                    ),
                })?;
            }
        }

        debug!("Updated metadata: {}", destination.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_request_creation() {
        let request = SyncRequest::new("source", "dest");

        assert_eq!(request.source, PathBuf::from("source"));
        assert_eq!(request.destination, PathBuf::from("dest"));
        assert!(request.options.incremental);
    }

    #[test]
    fn test_sync_options() {
        let incremental = SyncOptions::incremental();
        assert!(incremental.incremental);
        assert!(incremental.enable_delta);
        assert!(incremental.enable_caching);

        let full = SyncOptions::full();
        assert!(!full.incremental);
        assert!(!full.enable_delta);
        assert!(!full.enable_caching);

        let mirror = SyncOptions::mirror();
        assert!(mirror.delete_extra);
        assert!(mirror.incremental);
    }

    #[tokio::test]
    async fn test_sync_engine_creation() {
        let _engine = SyncEngine::new().await.unwrap();
        // Just test that it can be created without errors
        assert!(true);
    }
}
