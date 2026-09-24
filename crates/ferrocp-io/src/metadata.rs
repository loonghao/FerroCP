//! Metadata preservation shared by every copy engine
//!
//! Timestamps and permissions are preserved by all engines, not only the
//! buffered one. Each flag is independent, so a caller can ask for timestamps
//! without permissions or the other way round.
//!
//! See `docs/COPY_SEMANTICS.md` for what each platform actually preserves.

use crate::CopyOptions;
use ferrocp_types::{Error, Result};
use std::path::Path;

/// Preserve timestamps and/or permissions according to the options
///
/// Uses synchronous `std::fs` calls on purpose: this is a handful of metadata
/// syscalls, and doing so keeps the helper usable from both the async engines
/// and the synchronous micro-file engine.
///
/// # Errors
///
/// Returns an I/O error if the source metadata cannot be read or the
/// destination metadata cannot be updated. Permission failures on the
/// destination are **not** ignored: silently keeping the wrong permissions is
/// how a "preserve" flag becomes a lie.
pub fn preserve_metadata(source: &Path, destination: &Path, options: &CopyOptions) -> Result<()> {
    if !options.preserve_timestamps && !options.preserve_permissions {
        return Ok(());
    }

    let source_metadata = std::fs::metadata(source).map_err(|error| Error::Io {
        message: format!(
            "failed to read source metadata '{}': {}",
            source.display(),
            error
        ),
        kind: None,
    })?;

    if options.preserve_timestamps {
        preserve_timestamps(&source_metadata, destination)?;
    }

    if options.preserve_permissions {
        preserve_permissions(source, &source_metadata, destination)?;
    }

    Ok(())
}

/// Copy modification and access times from source to destination
fn preserve_timestamps(source_metadata: &std::fs::Metadata, destination: &Path) -> Result<()> {
    let accessed = source_metadata
        .accessed()
        .unwrap_or_else(|_| std::time::SystemTime::now());
    let modified = source_metadata
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    filetime::set_file_times(
        destination,
        filetime::FileTime::from_system_time(accessed),
        filetime::FileTime::from_system_time(modified),
    )
    .map_err(|error| Error::Io {
        message: format!(
            "failed to set file times on '{}': {}",
            destination.display(),
            error
        ),
        kind: None,
    })
}

/// Copy permission bits from source to destination, per platform
///
/// * Unix: the full mode is preserved.
/// * Windows: only the read-only attribute is preserved. ACLs and ownership are
///   never copied - this is a documented platform difference, not an omission.
fn preserve_permissions(
    source: &Path,
    source_metadata: &std::fs::Metadata,
    destination: &Path,
) -> Result<()> {
    #[cfg(unix)]
    {
        let permissions = source_metadata.permissions();
        std::fs::set_permissions(destination, permissions).map_err(|error| Error::Io {
            message: format!(
                "failed to set permissions on '{}': {}",
                destination.display(),
                error
            ),
            kind: Some(error.kind()),
        })?;
    }

    #[cfg(windows)]
    {
        // Windows has no POSIX mode. Mirror the read-only attribute, which is
        // the part of `std::fs::Permissions` that maps onto a file attribute,
        // and clear it when the source is writable so a stale read-only
        // destination does not survive a re-copy.
        let source_readonly = source_metadata.permissions().readonly();
        let dest_metadata = std::fs::metadata(destination).map_err(|error| Error::Io {
            message: format!(
                "failed to read destination metadata '{}': {}",
                destination.display(),
                error
            ),
            kind: None,
        })?;

        let mut permissions = dest_metadata.permissions();
        if permissions.readonly() != source_readonly {
            permissions.set_readonly(source_readonly);
            std::fs::set_permissions(destination, permissions).map_err(|error| Error::Io {
                message: format!(
                    "failed to set read-only attribute on '{}' (source '{}'): {}",
                    destination.display(),
                    source.display(),
                    error
                ),
                kind: None,
            })?;
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, source_metadata, destination);
        tracing::debug!(
            "permission preservation is not implemented on this platform: {} -> {}",
            source.display(),
            destination.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(path: &Path, contents: &str) {
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file.sync_all().unwrap();
    }

    fn mtime_secs(path: &Path) -> u64 {
        std::fs::metadata(path)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn timestamps_are_preserved_when_enabled() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        write_file(&source, "payload");
        write_file(&destination, "other");
        filetime::set_file_mtime(&source, filetime::FileTime::from_unix_time(1_234_567, 0))
            .unwrap();

        let options = CopyOptions::default();
        preserve_metadata(&source, &destination, &options).unwrap();

        assert_eq!(mtime_secs(&destination), 1_234_567);
    }

    #[test]
    fn timestamps_are_left_alone_when_disabled() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        write_file(&source, "payload");
        write_file(&destination, "other");
        filetime::set_file_mtime(&source, filetime::FileTime::from_unix_time(1_234_567, 0))
            .unwrap();
        filetime::set_file_mtime(&destination, filetime::FileTime::from_unix_time(999, 0)).unwrap();

        let mut options = CopyOptions::default();
        options.preserve_timestamps = false;
        preserve_metadata(&source, &destination, &options).unwrap();

        assert_eq!(mtime_secs(&destination), 999);
    }

    #[test]
    fn nothing_is_touched_when_both_are_disabled() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        write_file(&source, "payload");
        write_file(&destination, "other");
        filetime::set_file_mtime(&destination, filetime::FileTime::from_unix_time(999, 0)).unwrap();

        let mut options = CopyOptions::default();
        options.preserve_timestamps = false;
        options.preserve_permissions = false;
        preserve_metadata(&source, &destination, &options).unwrap();

        assert_eq!(mtime_secs(&destination), 999);
    }

    #[cfg(unix)]
    #[test]
    fn unix_mode_is_preserved_only_when_enabled() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest.txt");
        write_file(&source, "payload");
        write_file(&destination, "other");
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o750)).unwrap();

        let options = CopyOptions::default();
        preserve_metadata(&source, &destination, &options).unwrap();
        assert_eq!(
            std::fs::metadata(&destination)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o750
        );

        // Disabled: the destination mode must not change.
        let destination2 = dir.path().join("dest2.txt");
        write_file(&destination2, "other");
        std::fs::set_permissions(&destination2, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut options = CopyOptions::default();
        options.preserve_permissions = false;
        preserve_metadata(&source, &destination2, &options).unwrap();
        assert_eq!(
            std::fs::metadata(&destination2)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o644
        );
    }

    #[test]
    fn missing_source_is_reported() {
        let dir = TempDir::new().unwrap();
        let destination = dir.path().join("dest.txt");
        write_file(&destination, "other");

        let error = preserve_metadata(
            std::path::Path::new("/definitely/not/here"),
            &destination,
            &CopyOptions::default(),
        )
        .unwrap_err();

        assert!(
            error.to_string().contains("failed to read source metadata"),
            "error was: {error}"
        );
    }
}
