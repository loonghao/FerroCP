//! Symbolic link handling
//!
//! `FerroCP` used to skip symlinks silently, which loses data without any signal
//! to the caller. This module implements the two supported behaviours from
//! [`SymlinkMode`] plus the shared helpers the engines need.
//!
//! Platform differences are documented in `docs/COPY_SEMANTICS.md`. The two
//! that matter here:
//!
//! * On Unix a link is recreated with [`std::os::unix::fs::symlink`], which
//!   works for file links, directory links and dangling links alike.
//! * On Windows creating a symlink requires Developer Mode or an elevated
//!   process. When [`create_symlink`] fails it returns the underlying error so
//!   the caller can report it instead of dropping the entry.

use ferrocp_types::{Error, Result, SymlinkMode};
use std::path::{Path, PathBuf};

/// `true` when the metadata describes a symbolic link
///
/// Uses [`std::fs::symlink_metadata`] semantics: the caller must pass metadata
/// that was NOT obtained by following links.
#[must_use]
pub fn is_symlink(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Read the raw target of a symbolic link
///
/// The returned path is the link text as stored, not a resolved absolute path.
///
/// # Errors
///
/// Returns an error when `link` is not a symlink or cannot be read.
pub fn read_link(link: &Path) -> Result<PathBuf> {
    std::fs::read_link(link).map_err(|error| Error::Io {
        message: format!(
            "failed to read symbolic link '{}': {}",
            link.display(),
            error
        ),
        kind: None,
    })
}

/// Metadata of the link target, following the link
///
/// Returns an error for dangling links, which lets callers turn "link points
/// nowhere" into a reported failure instead of a silent skip.
///
/// # Errors
///
/// Propagates the underlying I/O error, including `NotFound` for dangling
/// links.
pub fn follow_link(link: &Path) -> Result<std::fs::Metadata> {
    std::fs::metadata(link).map_err(|error| Error::Io {
        message: format!(
            "failed to resolve symbolic link '{}': {}",
            link.display(),
            error
        ),
        kind: None,
    })
}

/// Recreate `link` at `destination`, preserving the raw link target
///
/// # Platform notes
///
/// * Unix: uses `symlink`, which handles file, directory and dangling links.
/// * Windows: uses `symlink_file` / `symlink_dir`. Dangling links cannot be
///   classified, so they are recreated as file links.
///
/// # Errors
///
/// Returns an error when the link cannot be read or created. On Windows this
/// commonly means the process lacks the `SeCreateSymbolicLinkPrivilege`
/// privilege (enable Developer Mode or run elevated).
pub fn create_symlink(link: &Path, destination: &Path) -> Result<()> {
    let target = read_link(link)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, destination).map_err(|error| Error::Io {
            message: format!(
                "failed to create symbolic link '{}' -> '{}': {}",
                destination.display(),
                target.display(),
                error
            ),
            kind: Some(error.kind()),
        })?;
    }

    #[cfg(windows)]
    {
        // A dangling link cannot be classified, so treat it as a file link.
        let target_is_dir = std::fs::metadata(link).is_ok_and(|metadata| metadata.is_dir());

        let result = if target_is_dir {
            std::os::windows::fs::symlink_dir(&target, destination)
        } else {
            std::os::windows::fs::symlink_file(&target, destination)
        };
        result.map_err(|error| Error::Io {
            message: format!(
                "failed to create symbolic link '{}' -> '{}': {} (creating symlinks on Windows \
                 requires Developer Mode or an elevated process)",
                destination.display(),
                target.display(),
                error
            ),
            kind: None,
        })?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        return Err(Error::other(format!(
            "recreating symbolic links is not supported on this platform: '{}'",
            link.display()
        )));
    }

    Ok(())
}

/// Copy the content a symlink points at to `destination`
///
/// Used by [`SymlinkMode::Follow`] when the link resolves to a regular file.
///
/// # Errors
///
/// Propagates I/O errors from reading the target and writing the destination.
pub fn copy_link_target(link: &Path, destination: &Path) -> Result<u64> {
    let bytes = std::fs::read(link).map_err(|error| Error::Io {
        message: format!(
            "failed to read symbolic link target '{}': {}",
            link.display(),
            error
        ),
        kind: None,
    })?;

    std::fs::write(destination, &bytes).map_err(|error| Error::Io {
        message: format!(
            "failed to write '{}' while copying link target '{}': {}",
            destination.display(),
            link.display(),
            error
        ),
        kind: None,
    })?;

    Ok(bytes.len() as u64)
}

/// Human-readable description of a symlink mode, for logs and error messages
#[must_use]
pub fn describe(mode: SymlinkMode) -> &'static str {
    match mode {
        SymlinkMode::Preserve => "recreate the symbolic link",
        SymlinkMode::Follow => "copy the content the link points at",
        SymlinkMode::Fail => "fail when a symbolic link is encountered",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a symlink, or report that this platform/process cannot
    ///
    /// Windows needs `SeCreateSymbolicLinkPrivilege` (Developer Mode or an
    /// elevated process). Tests degrade to a documented skip there instead of
    /// failing - and, just as importantly, instead of being cfg'd out so they
    /// never run on Windows CI at all.
    fn try_symlink(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(target, link);

        #[cfg(windows)]
        let result = {
            let is_dir = std::fs::metadata(target).is_ok_and(|metadata| metadata.is_dir());
            if is_dir {
                std::os::windows::fs::symlink_dir(target, link)
            } else {
                std::os::windows::fs::symlink_file(target, link)
            }
        };

        match result {
            Ok(()) => true,
            Err(error) => {
                eprintln!(
                    "skipping symlink test: cannot create '{}': {}",
                    link.display(),
                    error
                );
                false
            }
        }
    }

    #[test]
    fn create_symlink_recreates_file_link() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        let copy = dir.path().join("copy.txt");

        std::fs::write(&target, b"payload").unwrap();
        if !try_symlink(&target, &link) {
            return;
        }
        assert!(is_symlink(&std::fs::symlink_metadata(&link).unwrap()));

        create_symlink(&link, &copy).unwrap();
        assert!(is_symlink(&std::fs::symlink_metadata(&copy).unwrap()));
        assert_eq!(read_link(&copy).unwrap(), target);
        assert_eq!(std::fs::read(&copy).unwrap(), b"payload");
    }

    #[test]
    fn create_symlink_preserves_dangling_links() {
        let dir = TempDir::new().unwrap();
        let link = dir.path().join("dangling");
        let copy = dir.path().join("dangling_copy");

        if !try_symlink(Path::new("nowhere.txt"), &link) {
            return;
        }
        create_symlink(&link, &copy).unwrap();

        assert!(is_symlink(&std::fs::symlink_metadata(&copy).unwrap()));
        // The link is dangling, so resolving it must fail - and the caller can
        // report that instead of silently dropping the entry.
        assert!(follow_link(&copy).is_err());
    }

    #[test]
    fn follow_link_returns_target_metadata() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, b"0123456789").unwrap();
        if !try_symlink(&target, &link) {
            return;
        }

        let metadata = follow_link(&link).unwrap();
        assert_eq!(metadata.len(), 10);
        assert!(!is_symlink(&metadata), "follow_link must dereference");
    }

    #[test]
    fn copy_link_target_writes_the_dereferenced_content() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.bin");
        let link = dir.path().join("link.bin");
        let destination = dir.path().join("out.bin");

        std::fs::write(&target, b"dereferenced payload").unwrap();
        if !try_symlink(&target, &link) {
            return;
        }

        let payload = b"dereferenced payload";
        let bytes = copy_link_target(&link, &destination).unwrap();
        assert_eq!(bytes, payload.len() as u64);
        assert_eq!(std::fs::read(&destination).unwrap(), payload);
        assert!(
            !is_symlink(&std::fs::symlink_metadata(&destination).unwrap()),
            "following a link must produce a regular file"
        );
    }

    #[test]
    fn read_link_on_regular_file_errors() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("regular.txt");
        std::fs::write(&file, b"data").unwrap();

        assert!(read_link(&file).is_err());
        assert!(!is_symlink(&std::fs::metadata(&file).unwrap()));
    }

    #[test]
    fn describe_covers_every_mode() {
        for mode in [
            SymlinkMode::Preserve,
            SymlinkMode::Follow,
            SymlinkMode::Fail,
        ] {
            assert!(!describe(mode).is_empty());
        }
    }
}
