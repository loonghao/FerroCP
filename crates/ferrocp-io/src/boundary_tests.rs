//! Boundary tests for the files and paths a real copy has to survive
//!
//! The contract tests in `copy_contract_tests.rs` prove the *semantics* of a
//! copy (which policy wins, how a symlink is treated). This module proves the
//! copy still holds those semantics at the edges of what a filesystem accepts:
//! empty and large files, awkward path names, restricted permissions and a
//! destination that is the wrong kind of entry.
//!
//! These are **file level** boundaries, because that is what
//! [`BufferedCopyEngine`] copies. Directory trees are walked by
//! `ferrocp-engine`, so the tree-shaped boundaries (deep nesting, awkward names
//! inside a tree) live in `crates/ferrocp-engine/tests/boundary_tests.rs`.
//!
//! Every test here drives the real filesystem through the public
//! [`CopyEngine`] trait. Nothing is mocked: a copy that only works against a
//! fake filesystem says nothing about the paths above.

use crate::{BufferedCopyEngine, CopyEngine, CopyOptions};
use ferrocp_types::OverwritePolicy;
use std::io::Write;
use std::path::Path;
// `PathBuf` is only needed by the Unix cross-device test's cleanup guard.
#[cfg(unix)]
use std::path::PathBuf;
use tempfile::TempDir;

/// Options that keep the tests quiet and fast without changing the semantics
fn test_options() -> CopyOptions {
    CopyOptions {
        enable_progress: false,
        enable_preread: false,
        max_retries: 0,
        ..CopyOptions::default()
    }
}

fn write_bytes(path: &Path, contents: &[u8]) {
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(contents).unwrap();
    file.sync_all().unwrap();
}

fn write_file(path: &Path, contents: &str) {
    write_bytes(path, contents.as_bytes());
}

#[tokio::test]
async fn empty_file_is_copied_not_skipped() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("empty.bin");
    let destination = dir.path().join("empty-copy.bin");
    write_bytes(&source, b"");

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await
        .unwrap();

    assert!(destination.exists(), "an empty source must still be copied");
    assert_eq!(std::fs::read(&destination).unwrap().len(), 0);
    assert_eq!(stats.bytes_copied, 0);
    // An empty file is a file that was copied, not one that was skipped.
    assert_eq!(stats.files_copied, 1, "stats were: {stats:?}");
    assert_eq!(stats.files_skipped, 0, "stats were: {stats:?}");
}

/// A file has to survive the engine thresholds, not just the fast path.
///
/// 8 MiB is above the micro-file ceiling and large enough to force the buffered
/// engine through several buffer refills, which is where an off-by-one in the
/// read/write loop shows up. It stays small enough to keep CI honest about
/// runtime.
#[tokio::test]
async fn large_file_survives_several_buffer_refills() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("large.bin");
    let destination = dir.path().join("large-copy.bin");

    // A repeating pattern with a length that is not a power of two, so a copy
    // that misaligns its chunks cannot accidentally produce the same bytes.
    let payload: Vec<u8> = (0..8 * 1024 * 1024)
        .map(|i| u8::try_from(i % 251).unwrap())
        .collect();
    write_bytes(&source, &payload);

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await
        .unwrap();

    assert_eq!(std::fs::read(&destination).unwrap(), payload);
    assert_eq!(stats.bytes_copied, payload.len() as u64);
}

/// Paths with spaces, non-ASCII characters and characters that need escaping
/// have to round-trip byte for byte.
///
/// The names below cover Latin-1 accents, CJK, Cyrillic, an emoji, and the
/// quoting characters that break naive command construction. Each is copied on
/// its own so the assertion names exactly which one failed.
#[tokio::test]
async fn awkward_path_names_round_trip() {
    // No double quotes in these names: Windows rejects them outright, and the
    // point of the test is how FerroCP handles names the filesystem accepts.
    let names = [
        "plain with spaces.txt",
        "café-ünïcode.txt",
        "中文文件.txt",
        "файл.txt",
        "emoji-\u{1f4c1}-file.txt",
        "apostrophe's and [brackets].txt",
        "trailing space .txt",
        "semi;colon&amp.txt",
    ];

    let dir = TempDir::new().unwrap();
    // Parent directories with non-ASCII names too, not just the leaf files.
    let source_root = dir.path().join("源 directory");
    let destination_root = dir.path().join("целевая directory");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&destination_root).unwrap();

    let mut engine = BufferedCopyEngine::new();
    for (index, name) in names.iter().enumerate() {
        let source = source_root.join(name);
        let destination = destination_root.join(name);
        let payload = format!("payload {index}");
        write_file(&source, &payload);

        engine
            .copy_file_with_options(&source, &destination, test_options())
            .await
            .unwrap_or_else(|error| panic!("copying {name:?} failed: {error}"));

        assert!(
            destination.exists(),
            "{name:?} was not created at {destination:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&destination).unwrap(),
            payload,
            "{name:?} was copied with the wrong content"
        );
    }
}

/// A destination that exists as a directory must not be truncated into a file.
#[tokio::test]
async fn copying_onto_an_existing_directory_is_an_error() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("destination-as-directory");
    write_file(&source, "payload");
    std::fs::create_dir_all(&destination).unwrap();

    let mut engine = BufferedCopyEngine::new();
    let error = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await
        .expect_err("replacing a directory with a file must fail");

    assert!(
        destination.is_dir(),
        "the destination directory must be left alone"
    );
    assert!(
        !error.to_string().is_empty(),
        "the error must explain itself"
    );
}

/// An unreadable source is reported, never turned into an empty success.
///
/// Skipped on Windows because `std::fs::set_permissions` there only controls
/// the read-only attribute, which does not make a file unreadable, and skipped
/// when the test already runs as root, who can read anything. Neither case is
/// a pass: the restriction simply cannot be expressed, so the test says which
/// one it hit.
#[cfg(unix)]
#[tokio::test]
async fn unreadable_source_is_reported_not_swallowed() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        panic!(
            "this test cannot restrict access as root; run it as an unprivileged user to exercise \
             the permission-denied path"
        );
    }

    let dir = TempDir::new().unwrap();
    let source = dir.path().join("secret.txt");
    let destination = dir.path().join("copy.txt");
    write_file(&source, "classified");
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o000)).unwrap();

    let mut engine = BufferedCopyEngine::new();
    let result = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await;

    // Restore first so the TempDir can be cleaned up whatever happens below.
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o644)).unwrap();

    let error = result.expect_err("an unreadable source must be reported");
    assert!(
        !destination.exists(),
        "no destination may be produced from an unreadable source"
    );
    assert!(!error.to_string().is_empty());
}

/// A read-only destination directory is reported rather than silently left
/// half-populated.
#[cfg(unix)]
#[tokio::test]
async fn unwritable_destination_directory_is_reported() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        panic!(
            "this test cannot restrict access as root; run it as an unprivileged user to exercise \
             the permission-denied path"
        );
    }

    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    write_file(&source_root.join("file.txt"), "payload");

    let destination_root = dir.path().join("readonly-destination");
    std::fs::create_dir_all(&destination_root).unwrap();
    std::fs::set_permissions(&destination_root, std::fs::Permissions::from_mode(0o500)).unwrap();

    let mut engine = BufferedCopyEngine::new();
    let result = engine
        .copy_file_with_options(&source_root, &destination_root, test_options())
        .await;

    std::fs::set_permissions(&destination_root, std::fs::Permissions::from_mode(0o755)).unwrap();

    let error = result.expect_err("a read-only destination must be reported");
    assert!(!error.to_string().is_empty());
}

/// A genuine cross-device copy, run only where a second filesystem exists.
///
/// Unix compares `st_dev` to prove the two roots really are different devices,
/// so the name is only claimed when it is true. Hosted macOS runners and most
/// containers expose a single filesystem, so the common case is that no second
/// root is found.
///
/// That case is reported, not failed: whether a machine has a second filesystem
/// mounted is a property of the environment, not of FerroCP, and failing the
/// suite over it would turn every macOS run red for no reason. It is not a
/// silent pass either - the note names what went unverified, and
/// `cross_directory_copy_produces_identical_content` below still asserts the
/// behaviour that is defined on every platform.
#[cfg(unix)]
#[tokio::test]
async fn cross_device_copy_produces_identical_content() {
    use std::os::unix::fs::MetadataExt;

    let dir = TempDir::new().unwrap();
    let source = dir.path().join("cross-device.bin");
    let payload: Vec<u8> = (0..64 * 1024)
        .map(|i| u8::try_from(i % 97).unwrap())
        .collect();
    write_bytes(&source, &payload);

    let source_dev = std::fs::metadata(&source).unwrap().dev();
    let other_root = ["/dev/shm", "/run/shm", "/run", "/tmp"]
        .iter()
        .map(Path::new)
        .find(|root| match std::fs::metadata(root) {
            Ok(metadata) => metadata.is_dir() && metadata.dev() != source_dev,
            Err(_) => false,
        });

    let Some(other_root) = other_root else {
        // Printed rather than swallowed: a reader of the CI log should be able
        // to see that this case went unverified on this machine.
        eprintln!(
            "note: no second filesystem is mounted here, so a genuine cross-device copy was NOT \\
             exercised (the source is on device {source_dev}); mount a tmpfs other than the \\
             one holding {} to cover it",
            dir.path().display()
        );
        return;
    };

    let destination = other_root.join(format!("ferrocp-cross-device-{:?}.bin", std::process::id()));
    // Do not leave the artefact behind on a shared filesystem.
    let _cleanup = DeleteOnDrop(destination.clone());

    assert_ne!(
        source_dev,
        std::fs::metadata(other_root).unwrap().dev(),
        "the two roots are on the same device, so this is not a cross-device copy"
    );

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await
        .expect("a cross-device copy must succeed");

    assert_eq!(std::fs::read(&destination).unwrap(), payload);
    assert_eq!(stats.bytes_copied, payload.len() as u64);
}

/// The part of a cross-device copy that *is* defined, run on every platform.
///
/// `docs/COPY_SEMANTICS.md` §10 leaves cross-device semantics undefined, so
/// there is no device-specific behaviour to assert. What is defined is that the
/// bytes arrive intact however the two paths are placed, and that is worth
/// pinning everywhere - including on the platforms where the test above cannot
/// find a second filesystem.
#[tokio::test]
async fn cross_directory_copy_produces_identical_content() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source volume");
    let destination_root = dir.path().join("destination volume");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&destination_root).unwrap();

    let payload: Vec<u8> = (0..64 * 1024)
        .map(|i| u8::try_from(i % 97).unwrap())
        .collect();
    let source = source_root.join("cross-directory.bin");
    write_bytes(&source, &payload);
    let destination = destination_root.join("cross-directory.bin");

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, test_options())
        .await
        .unwrap();

    assert_eq!(std::fs::read(&destination).unwrap(), payload);
    assert_eq!(stats.bytes_copied, payload.len() as u64);
}

/// Removes a file created outside the test's `TempDir` when the test ends
//
// Only the cross-device test writes outside its `TempDir`, and that test is
// Unix-only.
#[cfg(unix)]
struct DeleteOnDrop(PathBuf);

#[cfg(unix)]
impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A destination that already exists is handled by the policy, and `fail` must
/// refuse without touching what is already there.
#[tokio::test]
async fn existing_destination_is_left_intact_under_the_fail_policy() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("destination.txt");
    write_file(&source, "new content");
    write_file(&destination, "original content");

    let options = CopyOptions {
        overwrite_policy: OverwritePolicy::Fail,
        ..test_options()
    };

    let mut engine = BufferedCopyEngine::new();
    engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .expect_err("the fail policy must refuse an existing destination");

    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "original content",
        "the fail policy must not modify the destination"
    );
}
