//! Overwrite policy enforcement
//!
//! This module is the single place where `FerroCP` decides whether an existing
//! destination may be replaced. Every copy engine consults
//! [`decide_overwrite`] before it opens the destination for writing, so no
//! engine can silently truncate a file the caller asked to keep.
//!
//! See `docs/COPY_SEMANTICS.md` for the user-facing contract.

use ferrocp_types::{Error, OverwriteDecision, OverwritePolicy, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

/// Callback invoked for [`OverwritePolicy::Prompt`]
///
/// The handler receives `(source, destination)` and returns the decision. It
/// must be safe to call from a worker thread.
pub type OverwritePromptFn = Arc<dyn Fn(&Path, &Path) -> OverwriteDecision + Send + Sync>;

/// Registered handler for [`OverwritePolicy::Prompt`]
///
/// `Prompt` without a handler is a configuration error rather than a silent
/// fallback: an unanswered prompt must never turn into "overwrite".
#[derive(Clone)]
pub struct OverwritePrompt {
    inner: OverwritePromptFn,
}

impl OverwritePrompt {
    /// Wrap a function or closure as a prompt handler
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(&Path, &Path) -> OverwriteDecision + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(handler),
        }
    }

    /// Ask the handler what to do with `destination`
    #[must_use]
    pub fn decide(&self, source: &Path, destination: &Path) -> OverwriteDecision {
        (self.inner)(source, destination)
    }
}

impl std::fmt::Debug for OverwritePrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OverwritePrompt(<callback>)")
    }
}

/// Decide whether an existing destination may be replaced
///
/// `source_metadata` is the metadata of the source file and is used by the
/// time- and content-sensitive policies. When the destination does not exist
/// the decision is always [`OverwriteDecision::Proceed`].
///
/// # Errors
///
/// - [`OverwritePolicy::Fail`] when the destination exists.
/// - [`OverwritePolicy::Prompt`] when no handler is registered.
/// - I/O errors while inspecting the destination.
pub fn decide_overwrite(
    source: &Path,
    source_metadata: &std::fs::Metadata,
    destination: &Path,
    policy: OverwritePolicy,
    prompt: Option<&OverwritePrompt>,
) -> Result<OverwriteDecision> {
    // Nothing to overwrite: every policy agrees on proceeding.
    let dest_metadata = match std::fs::symlink_metadata(destination) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(OverwriteDecision::Proceed);
        }
        Err(error) => {
            return Err(Error::Io {
                message: format!(
                    "failed to inspect destination '{}': {}",
                    destination.display(),
                    error
                ),
            });
        }
    };

    // Refuse to replace a directory with a file.
    if dest_metadata.is_dir() {
        return Err(Error::other(format!(
            "destination '{}' is an existing directory and cannot be replaced by a file",
            destination.display()
        )));
    }

    match policy {
        OverwritePolicy::Always => Ok(OverwriteDecision::Proceed),
        OverwritePolicy::Never => Ok(OverwriteDecision::Skip),
        OverwritePolicy::IfNewer => {
            let source_mtime = modified_time(source_metadata);
            let dest_mtime = modified_time(&dest_metadata);
            Ok(if newer_than(source_mtime, dest_mtime) {
                OverwriteDecision::Proceed
            } else {
                OverwriteDecision::Skip
            })
        }
        OverwritePolicy::IfDifferent => Ok(if differs(source_metadata, &dest_metadata) {
            OverwriteDecision::Proceed
        } else {
            OverwriteDecision::Skip
        }),
        OverwritePolicy::Fail => Err(Error::other(format!(
            "destination '{}' already exists and the overwrite policy is 'fail'",
            destination.display()
        ))),
        OverwritePolicy::Prompt => match prompt {
            Some(handler) => Ok(handler.decide(source, destination)),
            None => Err(Error::config(format!(
                "overwrite policy 'prompt' requires a prompt handler, but none was registered \
                 for '{}' -> '{}'",
                source.display(),
                destination.display()
            ))),
        },
    }
}

/// Extract the modification time, falling back to the epoch when unsupported
fn modified_time(metadata: &std::fs::Metadata) -> SystemTime {
    metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

/// `true` when `candidate` is strictly newer than `reference`
///
/// Equal timestamps are not newer, so `IfNewer` skips unchanged sources.
fn newer_than(candidate: SystemTime, reference: SystemTime) -> bool {
    match candidate.duration_since(reference) {
        Ok(elapsed) => elapsed > std::time::Duration::ZERO,
        // `candidate` is older than `reference`.
        Err(_) => false,
    }
}

/// `true` when size or modification time differ
fn differs(source: &std::fs::Metadata, destination: &std::fs::Metadata) -> bool {
    if source.len() != destination.len() {
        return true;
    }
    let source_mtime = modified_time(source);
    let dest_mtime = modified_time(destination);
    source_mtime != dest_mtime
}

/// Result of applying the overwrite policy to a single file copy
pub enum CopyGate {
    /// The copy may proceed
    Proceed,
    /// The copy must be skipped and reported as a skipped file
    Skip,
}

impl CopyGate {
    /// Apply the policy to a source/destination pair
    ///
    /// # Errors
    ///
    /// Propagates the errors documented on [`decide_overwrite`].
    #[must_use = "the gate tells the engine whether it may write"]
    pub fn evaluate(
        source: &Path,
        source_metadata: &std::fs::Metadata,
        destination: &Path,
        policy: OverwritePolicy,
        prompt: Option<&OverwritePrompt>,
    ) -> Result<Self> {
        match decide_overwrite(source, source_metadata, destination, policy, prompt)? {
            OverwriteDecision::Proceed => Ok(Self::Proceed),
            OverwriteDecision::Skip => Ok(Self::Skip),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(path: &Path, contents: &[u8]) -> std::fs::Metadata {
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(contents).unwrap();
        file.sync_all().unwrap();
        drop(file);
        std::fs::metadata(path).unwrap()
    }

    fn set_mtime(path: &Path, offset_secs: i64) {
        let time = filetime::FileTime::from_unix_time(offset_secs, 0);
        filetime::set_file_mtime(path, time).unwrap();
    }

    #[test]
    fn missing_destination_always_proceeds() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("missing.txt");
        let metadata = write_file(&source, b"data");

        for policy in [
            OverwritePolicy::Always,
            OverwritePolicy::Never,
            OverwritePolicy::IfNewer,
            OverwritePolicy::IfDifferent,
            OverwritePolicy::Fail,
        ] {
            let decision =
                decide_overwrite(&source, &metadata, &destination, policy, None).unwrap();
            assert_eq!(
                decision,
                OverwriteDecision::Proceed,
                "policy {policy:?} must not block a missing destination"
            );
        }
    }

    #[test]
    fn policy_never_skips_existing_destination() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        let metadata = write_file(&source, b"new");
        write_file(&destination, b"old");

        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Never,
            None,
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Skip);
    }

    #[test]
    fn policy_fail_errors_on_existing_destination() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        let metadata = write_file(&source, b"new");
        write_file(&destination, b"old");

        let error = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Fail,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn policy_if_newer_compares_modification_times() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        let metadata = write_file(&source, b"new");

        write_file(&destination, b"old");
        set_mtime(&destination, 1_000_000);
        set_mtime(&source, 2_000_000);

        let newer = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::IfNewer,
            None,
        )
        .unwrap();
        assert_eq!(newer, OverwriteDecision::Proceed);

        set_mtime(&source, 500_000);
        let metadata = std::fs::metadata(&source).unwrap();
        let older = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::IfNewer,
            None,
        )
        .unwrap();
        assert_eq!(older, OverwriteDecision::Skip);
    }

    #[test]
    fn policy_if_different_compares_size_and_time() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");

        // Same size, same mtime -> skip
        write_file(&source, b"same-size!");
        write_file(&destination, b"same-size!");
        set_mtime(&source, 1_500_000);
        set_mtime(&destination, 1_500_000);
        let metadata = std::fs::metadata(&source).unwrap();
        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::IfDifferent,
            None,
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Skip);

        // Different mtime, same size -> proceed
        set_mtime(&source, 1_600_000);
        let metadata = std::fs::metadata(&source).unwrap();
        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::IfDifferent,
            None,
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Proceed);

        // Different size -> proceed
        let metadata = write_file(&source, b"a much longer payload");
        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::IfDifferent,
            None,
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Proceed);
    }

    #[test]
    fn policy_prompt_without_handler_is_a_config_error() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        let metadata = write_file(&source, b"new");
        write_file(&destination, b"old");

        let error = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Prompt,
            None,
        )
        .unwrap_err();
        assert_eq!(error.kind(), ferrocp_types::ErrorKind::Config);
        assert!(error.to_string().contains("requires a prompt handler"));
    }

    #[test]
    fn policy_prompt_uses_the_registered_handler() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        let metadata = write_file(&source, b"new");
        write_file(&destination, b"old");

        let skip = OverwritePrompt::new(|_source, _destination| OverwriteDecision::Skip);
        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Prompt,
            Some(&skip),
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Skip);

        let proceed = OverwritePrompt::new(|_source, _destination| OverwriteDecision::Proceed);
        let decision = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Prompt,
            Some(&proceed),
        )
        .unwrap();
        assert_eq!(decision, OverwriteDecision::Proceed);
    }

    #[test]
    fn existing_directory_destination_is_never_truncated() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest_dir");
        let metadata = write_file(&source, b"new");
        std::fs::create_dir(&destination).unwrap();

        let error = decide_overwrite(
            &source,
            &metadata,
            &destination,
            OverwritePolicy::Always,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("existing directory"));
    }

    #[test]
    fn newer_than_handles_equal_timestamps() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
        assert!(!newer_than(now, now));
        assert!(newer_than(now + std::time::Duration::from_secs(1), now));
        assert!(!newer_than(now, now + std::time::Duration::from_secs(1)));
    }
}
