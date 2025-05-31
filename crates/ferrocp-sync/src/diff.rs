//! File difference detection and analysis

use ferrocp_types::{Error, FileMetadata, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, info};

/// Type of change detected in a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was moved/renamed
    Moved,
    /// File metadata changed (permissions, timestamps, etc.)
    MetadataChanged,
}

/// Information about a file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path to the file
    pub path: PathBuf,
    /// Type of change
    pub change_type: ChangeType,
    /// Old path (for moved files)
    pub old_path: Option<PathBuf>,
    /// File size
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// File hash (if available)
    pub hash: Option<String>,
    /// Whether this is a directory
    pub is_directory: bool,
}

impl FileChange {
    /// Create a new file change
    pub fn new(path: PathBuf, change_type: ChangeType) -> Self {
        Self {
            path,
            change_type,
            old_path: None,
            size: 0,
            modified: SystemTime::UNIX_EPOCH,
            hash: None,
            is_directory: false,
        }
    }

    /// Set the old path for moved files
    pub fn with_old_path(mut self, old_path: PathBuf) -> Self {
        self.old_path = Some(old_path);
        self
    }

    /// Set file metadata
    pub fn with_metadata(mut self, metadata: &FileMetadata) -> Self {
        self.size = metadata.size;
        self.modified = metadata.modified;
        self.is_directory = metadata.is_dir;
        self
    }

    /// Set file hash
    pub fn with_hash(mut self, hash: String) -> Self {
        self.hash = Some(hash);
        self
    }
}

/// Configuration for difference detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Whether to detect moved files
    pub detect_moves: bool,
    /// Whether to check file content hashes
    pub check_content_hash: bool,
    /// Whether to check metadata changes
    pub check_metadata: bool,
    /// Minimum file size to check content hash
    pub min_hash_size: u64,
    /// Maximum file size to check content hash
    pub max_hash_size: u64,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            detect_moves: true,
            check_content_hash: true,
            check_metadata: true,
            min_hash_size: 1024,              // 1KB
            max_hash_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// File information for difference detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// File hash
    pub hash: Option<String>,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File permissions
    pub permissions: u32,
}

impl FileInfo {
    /// Create file info from metadata
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let metadata = fs::metadata(path).await.map_err(|e| Error::Io {
            message: format!("Failed to get metadata for '{}': {}", path.display(), e),
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            hash: None,
            is_directory: metadata.is_dir(),
            permissions: 0, // TODO: Get actual permissions
        })
    }

    /// Check if this file info differs from another
    pub fn differs_from(&self, other: &Self, config: &DiffConfig) -> bool {
        // Check size
        if self.size != other.size {
            return true;
        }

        // Check modification time
        if self.modified != other.modified {
            return true;
        }

        // Check metadata if enabled
        if config.check_metadata && self.permissions != other.permissions {
            return true;
        }

        // Check content hash if available and enabled
        if config.check_content_hash {
            if let (Some(hash1), Some(hash2)) = (&self.hash, &other.hash) {
                return hash1 != hash2;
            }
        }

        false
    }
}

/// Engine for detecting file differences
#[derive(Debug)]
pub struct DiffEngine {
    config: DiffConfig,
}

impl DiffEngine {
    /// Create a new diff engine
    pub fn new(config: DiffConfig) -> Self {
        Self { config }
    }

    /// Create a diff engine with default configuration
    pub fn default() -> Self {
        Self::new(DiffConfig::default())
    }

    /// Scan a directory and return file information
    pub async fn scan_directory<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<HashMap<PathBuf, FileInfo>> {
        let path = path.as_ref();
        let mut files = HashMap::new();

        self.scan_directory_recursive(path, path, &mut files)
            .await?;

        info!("Scanned {} files in '{}'", files.len(), path.display());
        Ok(files)
    }

    /// Recursively scan directory
    fn scan_directory_recursive<'a>(
        &'a self,
        base_path: &'a Path,
        current_path: &'a Path,
        files: &'a mut HashMap<PathBuf, FileInfo>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            let mut entries = fs::read_dir(current_path).await.map_err(|e| Error::Io {
                message: format!(
                    "Failed to read directory '{}': {}",
                    current_path.display(),
                    e
                ),
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| Error::Io {
                message: format!("Failed to read directory entry: {}", e),
            })? {
                let entry_path = entry.path();
                let relative_path = entry_path.strip_prefix(base_path).unwrap_or(&entry_path);

                let metadata = entry.metadata().await.map_err(|e| Error::Io {
                    message: format!(
                        "Failed to get metadata for '{}': {}",
                        entry_path.display(),
                        e
                    ),
                })?;

                // Skip symlinks if not following them
                if metadata.file_type().is_symlink() && !self.config.follow_symlinks {
                    debug!("Skipping symlink: {}", entry_path.display());
                    continue;
                }

                let mut file_info = FileInfo {
                    path: relative_path.to_path_buf(),
                    size: metadata.len(),
                    modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    hash: None,
                    is_directory: metadata.is_dir(),
                    permissions: 0, // TODO: Get actual permissions
                };

                // Calculate hash for files if enabled
                if !file_info.is_directory
                    && self.config.check_content_hash
                    && file_info.size >= self.config.min_hash_size
                    && file_info.size <= self.config.max_hash_size
                {
                    file_info.hash = Some(self.calculate_file_hash(&entry_path).await?);
                }

                files.insert(relative_path.to_path_buf(), file_info);

                // Recurse into directories
                if metadata.is_dir() {
                    self.scan_directory_recursive(base_path, &entry_path, files)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Calculate hash for a file
    async fn calculate_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let path = path.as_ref();
        let content = fs::read(path).await.map_err(|e| Error::Io {
            message: format!("Failed to read file '{}': {}", path.display(), e),
        })?;

        let hash = blake3::hash(&content);
        Ok(hash.to_hex().to_string())
    }

    /// Compare two file maps and detect changes
    pub fn detect_changes(
        &self,
        source_files: &HashMap<PathBuf, FileInfo>,
        dest_files: &HashMap<PathBuf, FileInfo>,
    ) -> Vec<FileChange> {
        let mut changes = Vec::new();

        // Find added and modified files
        for (path, source_info) in source_files {
            match dest_files.get(path) {
                Some(dest_info) => {
                    // File exists in both, check if modified
                    if source_info.differs_from(dest_info, &self.config) {
                        let change = FileChange::new(path.clone(), ChangeType::Modified)
                            .with_metadata(&FileMetadata {
                                size: source_info.size,
                                modified: source_info.modified,
                                created: None,
                                permissions: source_info.permissions,
                                is_dir: source_info.is_directory,
                                is_symlink: false,
                            });
                        changes.push(change);
                    }
                }
                None => {
                    // File only exists in source, it's added
                    let change = FileChange::new(path.clone(), ChangeType::Added).with_metadata(
                        &FileMetadata {
                            size: source_info.size,
                            modified: source_info.modified,
                            created: None,
                            permissions: source_info.permissions,
                            is_dir: source_info.is_directory,
                            is_symlink: false,
                        },
                    );
                    changes.push(change);
                }
            }
        }

        // Find deleted files
        for (path, dest_info) in dest_files {
            if !source_files.contains_key(path) {
                let change = FileChange::new(path.clone(), ChangeType::Deleted).with_metadata(
                    &FileMetadata {
                        size: dest_info.size,
                        modified: dest_info.modified,
                        created: None,
                        permissions: dest_info.permissions,
                        is_dir: dest_info.is_directory,
                        is_symlink: false,
                    },
                );
                changes.push(change);
            }
        }

        // Detect moved files if enabled
        if self.config.detect_moves {
            changes = self.detect_moves(changes, source_files, dest_files);
        }

        info!("Detected {} changes", changes.len());
        changes
    }

    /// Detect moved files by matching hashes
    fn detect_moves(
        &self,
        mut changes: Vec<FileChange>,
        source_files: &HashMap<PathBuf, FileInfo>,
        dest_files: &HashMap<PathBuf, FileInfo>,
    ) -> Vec<FileChange> {
        if !self.config.check_content_hash {
            return changes;
        }

        let mut hash_to_source: HashMap<String, &PathBuf> = HashMap::new();
        let mut hash_to_dest: HashMap<String, &PathBuf> = HashMap::new();

        // Build hash maps
        for (path, info) in source_files {
            if let Some(hash) = &info.hash {
                hash_to_source.insert(hash.clone(), path);
            }
        }

        for (path, info) in dest_files {
            if let Some(hash) = &info.hash {
                hash_to_dest.insert(hash.clone(), path);
            }
        }

        // Find potential moves
        let mut moves = Vec::new();
        let mut to_remove = Vec::new();

        for (i, change) in changes.iter().enumerate() {
            if change.change_type == ChangeType::Added {
                if let Some(source_info) = source_files.get(&change.path) {
                    if let Some(hash) = &source_info.hash {
                        if let Some(old_path) = hash_to_dest.get(hash) {
                            // Found a potential move
                            let move_change =
                                FileChange::new(change.path.clone(), ChangeType::Moved)
                                    .with_old_path((*old_path).clone())
                                    .with_metadata(&FileMetadata {
                                        size: source_info.size,
                                        modified: source_info.modified,
                                        created: None,
                                        permissions: source_info.permissions,
                                        is_dir: source_info.is_directory,
                                        is_symlink: false,
                                    });
                            moves.push(move_change);
                            to_remove.push(i);
                        }
                    }
                }
            }
        }

        // Remove the changes that are actually moves
        for &index in to_remove.iter().rev() {
            changes.remove(index);
        }

        // Add the move changes
        changes.extend(moves);

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_file_info_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").await.unwrap();

        let file_info = FileInfo::from_path(&file_path).await.unwrap();
        assert_eq!(file_info.path, file_path);
        assert_eq!(file_info.size, 12);
        assert!(!file_info.is_directory);
    }

    #[tokio::test]
    async fn test_diff_engine_scan() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").await.unwrap();

        let engine = DiffEngine::default();
        let files = engine.scan_directory(temp_dir.path()).await.unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains_key(Path::new("test.txt")));
    }

    #[test]
    fn test_change_detection() {
        let mut source_files = HashMap::new();
        let dest_files = HashMap::new();

        // Add a file that exists in source but not dest
        source_files.insert(
            PathBuf::from("new_file.txt"),
            FileInfo {
                path: PathBuf::from("new_file.txt"),
                size: 100,
                modified: SystemTime::now(),
                hash: None,
                is_directory: false,
                permissions: 0,
            },
        );

        let engine = DiffEngine::default();
        let changes = engine.detect_changes(&source_files, &dest_files);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Added);
        assert_eq!(changes[0].path, PathBuf::from("new_file.txt"));
    }
}
