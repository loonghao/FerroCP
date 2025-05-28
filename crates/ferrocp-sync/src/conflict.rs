//! Conflict resolution for file synchronization

use ferrocp_types::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, info};

/// Type of conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// Both source and destination files were modified
    BothModified,
    /// File exists in destination but not in source (and destination is newer)
    DestinationNewer,
    /// File was deleted in source but modified in destination
    DeletedInSourceModifiedInDest,
    /// File was modified in source but deleted in destination
    ModifiedInSourceDeletedInDest,
    /// File type conflict (file vs directory)
    TypeMismatch,
    /// Permission conflict
    PermissionConflict,
}

/// Resolution strategy for conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use source file (overwrite destination)
    UseSource,
    /// Use destination file (skip source)
    UseDestination,
    /// Create backup of destination and use source
    BackupAndUseSource,
    /// Rename source file and keep both
    KeepBoth,
    /// Skip this file entirely
    Skip,
    /// Ask user for resolution
    AskUser,
}

/// Information about a file conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflict {
    /// Path where conflict occurred
    pub path: PathBuf,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Source file information
    pub source_info: Option<ConflictFileInfo>,
    /// Destination file information
    pub destination_info: Option<ConflictFileInfo>,
    /// Suggested resolution
    pub suggested_resolution: ConflictResolution,
    /// Whether this conflict has been resolved
    pub resolved: bool,
    /// Chosen resolution
    pub resolution: Option<ConflictResolution>,
}

/// File information for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFileInfo {
    /// File size
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// File hash (if available)
    pub hash: Option<String>,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File permissions
    pub permissions: u32,
}

impl FileConflict {
    /// Create a new file conflict
    pub fn new(path: PathBuf, conflict_type: ConflictType) -> Self {
        let suggested_resolution = match conflict_type {
            ConflictType::BothModified => ConflictResolution::AskUser,
            ConflictType::DestinationNewer => ConflictResolution::UseDestination,
            ConflictType::DeletedInSourceModifiedInDest => ConflictResolution::AskUser,
            ConflictType::ModifiedInSourceDeletedInDest => ConflictResolution::UseSource,
            ConflictType::TypeMismatch => ConflictResolution::AskUser,
            ConflictType::PermissionConflict => ConflictResolution::UseSource,
        };

        Self {
            path,
            conflict_type,
            source_info: None,
            destination_info: None,
            suggested_resolution,
            resolved: false,
            resolution: None,
        }
    }

    /// Set source file information
    pub fn with_source_info(mut self, info: ConflictFileInfo) -> Self {
        self.source_info = Some(info);
        self
    }

    /// Set destination file information
    pub fn with_destination_info(mut self, info: ConflictFileInfo) -> Self {
        self.destination_info = Some(info);
        self
    }

    /// Resolve the conflict with a specific resolution
    pub fn resolve(&mut self, resolution: ConflictResolution) {
        self.resolution = Some(resolution);
        self.resolved = true;
        debug!(
            "Resolved conflict for '{}' with {:?}",
            self.path.display(),
            resolution
        );
    }

    /// Check if this conflict can be auto-resolved
    pub fn can_auto_resolve(&self) -> bool {
        matches!(
            self.suggested_resolution,
            ConflictResolution::UseSource
                | ConflictResolution::UseDestination
                | ConflictResolution::BackupAndUseSource
                | ConflictResolution::Skip
        )
    }

    /// Get a human-readable description of the conflict
    pub fn description(&self) -> String {
        match self.conflict_type {
            ConflictType::BothModified => {
                format!(
                    "File '{}' was modified in both source and destination",
                    self.path.display()
                )
            }
            ConflictType::DestinationNewer => {
                format!(
                    "Destination file '{}' is newer than source",
                    self.path.display()
                )
            }
            ConflictType::DeletedInSourceModifiedInDest => {
                format!(
                    "File '{}' was deleted in source but modified in destination",
                    self.path.display()
                )
            }
            ConflictType::ModifiedInSourceDeletedInDest => {
                format!(
                    "File '{}' was modified in source but deleted in destination",
                    self.path.display()
                )
            }
            ConflictType::TypeMismatch => {
                format!(
                    "Type mismatch for '{}' (file vs directory)",
                    self.path.display()
                )
            }
            ConflictType::PermissionConflict => {
                format!("Permission conflict for '{}'", self.path.display())
            }
        }
    }
}

/// Configuration for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictConfig {
    /// Default resolution for auto-resolvable conflicts
    pub default_resolution: ConflictResolution,
    /// Whether to create backups when overwriting files
    pub create_backups: bool,
    /// Backup file suffix
    pub backup_suffix: String,
    /// Whether to auto-resolve conflicts when possible
    pub auto_resolve: bool,
    /// Whether to prefer newer files in conflicts
    pub prefer_newer: bool,
    /// Whether to prefer larger files in conflicts
    pub prefer_larger: bool,
}

impl Default for ConflictConfig {
    fn default() -> Self {
        Self {
            default_resolution: ConflictResolution::AskUser,
            create_backups: true,
            backup_suffix: ".backup".to_string(),
            auto_resolve: false,
            prefer_newer: true,
            prefer_larger: false,
        }
    }
}

/// Conflict resolver for handling file synchronization conflicts
#[derive(Debug)]
pub struct ConflictResolver {
    config: ConflictConfig,
    conflicts: Vec<FileConflict>,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(config: ConflictConfig) -> Self {
        Self {
            config,
            conflicts: Vec::new(),
        }
    }

    /// Create a conflict resolver with default configuration
    pub fn default() -> Self {
        Self::new(ConflictConfig::default())
    }

    /// Add a conflict to be resolved
    pub fn add_conflict(&mut self, conflict: FileConflict) {
        info!("Added conflict: {}", conflict.description());
        self.conflicts.push(conflict);
    }

    /// Get all unresolved conflicts
    pub fn get_unresolved_conflicts(&self) -> Vec<&FileConflict> {
        self.conflicts.iter().filter(|c| !c.resolved).collect()
    }

    /// Get all resolved conflicts
    pub fn get_resolved_conflicts(&self) -> Vec<&FileConflict> {
        self.conflicts.iter().filter(|c| c.resolved).collect()
    }

    /// Auto-resolve conflicts based on configuration
    pub fn auto_resolve_conflicts(&mut self) -> usize {
        if !self.config.auto_resolve {
            return 0;
        }

        let mut resolved_count = 0;
        let mut resolutions = Vec::new();

        // First pass: determine resolutions
        for (index, conflict) in self.conflicts.iter().enumerate() {
            if conflict.resolved {
                continue;
            }

            let resolution = self.determine_auto_resolution(conflict);
            if let Some(res) = resolution {
                resolutions.push((index, res));
            }
        }

        // Second pass: apply resolutions
        for (index, resolution) in resolutions {
            if let Some(conflict) = self.conflicts.get_mut(index) {
                conflict.resolve(resolution);
                resolved_count += 1;
            }
        }

        if resolved_count > 0 {
            info!("Auto-resolved {} conflicts", resolved_count);
        }

        resolved_count
    }

    /// Determine automatic resolution for a conflict
    fn determine_auto_resolution(&self, conflict: &FileConflict) -> Option<ConflictResolution> {
        match conflict.conflict_type {
            ConflictType::BothModified => {
                if self.config.prefer_newer {
                    self.resolve_by_timestamp(conflict)
                } else if self.config.prefer_larger {
                    self.resolve_by_size(conflict)
                } else {
                    None
                }
            }
            ConflictType::DestinationNewer => {
                if self.config.prefer_newer {
                    Some(ConflictResolution::UseDestination)
                } else {
                    Some(ConflictResolution::UseSource)
                }
            }
            ConflictType::ModifiedInSourceDeletedInDest => Some(ConflictResolution::UseSource),
            ConflictType::DeletedInSourceModifiedInDest => Some(ConflictResolution::UseDestination),
            ConflictType::PermissionConflict => Some(ConflictResolution::UseSource),
            ConflictType::TypeMismatch => {
                None // Always require manual resolution
            }
        }
    }

    /// Resolve conflict by timestamp (prefer newer)
    fn resolve_by_timestamp(&self, conflict: &FileConflict) -> Option<ConflictResolution> {
        match (&conflict.source_info, &conflict.destination_info) {
            (Some(source), Some(dest)) => {
                if source.modified > dest.modified {
                    Some(ConflictResolution::UseSource)
                } else if dest.modified > source.modified {
                    Some(ConflictResolution::UseDestination)
                } else {
                    // Same timestamp, fall back to size
                    self.resolve_by_size(conflict)
                }
            }
            _ => None,
        }
    }

    /// Resolve conflict by file size (prefer larger)
    fn resolve_by_size(&self, conflict: &FileConflict) -> Option<ConflictResolution> {
        match (&conflict.source_info, &conflict.destination_info) {
            (Some(source), Some(dest)) => {
                if source.size > dest.size {
                    Some(ConflictResolution::UseSource)
                } else if dest.size > source.size {
                    Some(ConflictResolution::UseDestination)
                } else {
                    // Same size, use default
                    Some(self.config.default_resolution)
                }
            }
            _ => None,
        }
    }

    /// Manually resolve a specific conflict
    pub fn resolve_conflict(
        &mut self,
        path: &PathBuf,
        resolution: ConflictResolution,
    ) -> Result<()> {
        let conflict = self
            .conflicts
            .iter_mut()
            .find(|c| &c.path == path && !c.resolved)
            .ok_or_else(|| Error::Sync {
                message: format!("Conflict not found for path: {}", path.display()),
            })?;

        conflict.resolve(resolution);
        Ok(())
    }

    /// Resolve all conflicts with a specific resolution
    pub fn resolve_all_conflicts(&mut self, resolution: ConflictResolution) {
        let mut resolved_count = 0;

        for conflict in &mut self.conflicts {
            if !conflict.resolved {
                conflict.resolve(resolution);
                resolved_count += 1;
            }
        }

        info!(
            "Resolved {} conflicts with {:?}",
            resolved_count, resolution
        );
    }

    /// Clear all resolved conflicts
    pub fn clear_resolved_conflicts(&mut self) {
        let initial_count = self.conflicts.len();
        self.conflicts.retain(|c| !c.resolved);
        let removed_count = initial_count - self.conflicts.len();

        if removed_count > 0 {
            debug!("Cleared {} resolved conflicts", removed_count);
        }
    }

    /// Get conflict statistics
    pub fn get_stats(&self) -> ConflictStats {
        let total_conflicts = self.conflicts.len();
        let resolved_conflicts = self.conflicts.iter().filter(|c| c.resolved).count();
        let unresolved_conflicts = total_conflicts - resolved_conflicts;

        let mut conflict_types = std::collections::HashMap::new();
        for conflict in &self.conflicts {
            *conflict_types.entry(conflict.conflict_type).or_insert(0) += 1;
        }

        ConflictStats {
            total_conflicts,
            resolved_conflicts,
            unresolved_conflicts,
            conflict_types,
        }
    }
}

/// Conflict resolution statistics
#[derive(Debug, Clone)]
pub struct ConflictStats {
    /// Total number of conflicts
    pub total_conflicts: usize,
    /// Number of resolved conflicts
    pub resolved_conflicts: usize,
    /// Number of unresolved conflicts
    pub unresolved_conflicts: usize,
    /// Count of each conflict type
    pub conflict_types: std::collections::HashMap<ConflictType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_creation() {
        let conflict = FileConflict::new(PathBuf::from("test.txt"), ConflictType::BothModified);

        assert_eq!(conflict.path, PathBuf::from("test.txt"));
        assert_eq!(conflict.conflict_type, ConflictType::BothModified);
        assert!(!conflict.resolved);
        assert_eq!(conflict.suggested_resolution, ConflictResolution::AskUser);
    }

    #[test]
    fn test_conflict_resolution() {
        let mut conflict = FileConflict::new(PathBuf::from("test.txt"), ConflictType::BothModified);

        assert!(!conflict.resolved);

        conflict.resolve(ConflictResolution::UseSource);

        assert!(conflict.resolved);
        assert_eq!(conflict.resolution, Some(ConflictResolution::UseSource));
    }

    #[test]
    fn test_conflict_resolver() {
        let mut resolver = ConflictResolver::default();

        let conflict = FileConflict::new(
            PathBuf::from("test.txt"),
            ConflictType::ModifiedInSourceDeletedInDest,
        );

        resolver.add_conflict(conflict);

        assert_eq!(resolver.get_unresolved_conflicts().len(), 1);
        assert_eq!(resolver.get_resolved_conflicts().len(), 0);

        // Test auto-resolution
        let mut config = ConflictConfig::default();
        config.auto_resolve = true;
        resolver.config = config;

        let resolved_count = resolver.auto_resolve_conflicts();
        assert_eq!(resolved_count, 1);
        assert_eq!(resolver.get_unresolved_conflicts().len(), 0);
        assert_eq!(resolver.get_resolved_conflicts().len(), 1);
    }
}
