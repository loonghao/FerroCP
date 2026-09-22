//! End-to-end tests for the copy-semantics contract
//!
//! These live in their own module because they exercise the public
//! [`CopyEngine`] trait end to end, which is the only way to prove the policy is
//! honoured by a real copy rather than only by the decision helper.
//!
//! See `docs/COPY_SEMANTICS.md` for the normative contract.

use crate::policy::OverwritePrompt;
use crate::{BufferedCopyEngine, CopyEngine, CopyOptions};
use ferrocp_types::{ErrorKind, OverwriteDecision, OverwritePolicy};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn write_file(path: &Path, contents: &str) -> std::fs::Metadata {
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    file.sync_all().unwrap();
    drop(file);
    std::fs::metadata(path).unwrap()
}

fn set_mtime(path: &Path, secs: i64) {
    filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(secs, 0)).unwrap();
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

fn options_with(policy: OverwritePolicy) -> CopyOptions {
    CopyOptions {
        overwrite_policy: policy,
        // Keep the tests fast and quiet.
        enable_progress: false,
        enable_preread: false,
        max_retries: 0,
        ..CopyOptions::default()
    }
}

#[tokio::test]
async fn always_policy_replaces_existing_destination() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, options_with(OverwritePolicy::Always))
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.files_skipped, 0);
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "new-content"
    );
}

#[tokio::test]
async fn never_policy_keeps_existing_destination() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, options_with(OverwritePolicy::Never))
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 0);
    assert_eq!(stats.files_skipped, 1, "the skip must be observable");
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "old");
}

#[tokio::test]
async fn if_newer_policy_compares_timestamps() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "newer");
    write_file(&destination, "older");

    set_mtime(&destination, 1_000_000);
    set_mtime(&source, 2_000_000);

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(
            &source,
            &destination,
            options_with(OverwritePolicy::IfNewer),
        )
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "newer");

    // Make the source older than the destination: now it must be skipped.
    set_mtime(&source, 500_000);
    let stats = engine
        .copy_file_with_options(
            &source,
            &destination,
            options_with(OverwritePolicy::IfNewer),
        )
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 0);
    assert_eq!(stats.files_skipped, 1);
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "newer",
        "the destination must keep the content written above"
    );
}

#[tokio::test]
async fn if_different_policy_skips_identical_files() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "identical");
    write_file(&destination, "identical");
    set_mtime(&source, 1_500_000);
    set_mtime(&destination, 1_500_000);

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(
            &source,
            &destination,
            options_with(OverwritePolicy::IfDifferent),
        )
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 0);
    assert_eq!(stats.files_skipped, 1);

    // A different size makes them differ.
    write_file(&source, "a longer payload");
    set_mtime(&source, 1_500_000);
    let stats = engine
        .copy_file_with_options(
            &source,
            &destination,
            options_with(OverwritePolicy::IfDifferent),
        )
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "a longer payload"
    );
}

#[tokio::test]
async fn fail_policy_errors_without_touching_the_destination() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut engine = BufferedCopyEngine::new();
    let error = engine
        .copy_file_with_options(&source, &destination, options_with(OverwritePolicy::Fail))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "old",
        "the destination must not be truncated"
    );
}

#[tokio::test]
async fn prompt_policy_without_handler_is_a_config_error() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut engine = BufferedCopyEngine::new();
    let error = engine
        .copy_file_with_options(&source, &destination, options_with(OverwritePolicy::Prompt))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Config);
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "old");
}

#[tokio::test]
async fn prompt_policy_defers_to_the_handler() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut options = options_with(OverwritePolicy::Prompt);
    options.overwrite_prompt = Some(OverwritePrompt::new(|_source, _destination| {
        OverwriteDecision::Proceed
    }));

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        "new-content"
    );
}

#[tokio::test]
async fn prompt_handler_can_decline() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "new-content");
    write_file(&destination, "old");

    let mut options = options_with(OverwritePolicy::Prompt);
    options.overwrite_prompt = Some(OverwritePrompt::new(|_source, _destination| {
        OverwriteDecision::Skip
    }));

    let mut engine = BufferedCopyEngine::new();
    let stats = engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .unwrap();

    assert_eq!(stats.files_skipped, 1);
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "old");
}

#[tokio::test]
async fn timestamps_are_preserved_when_requested() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "payload");
    set_mtime(&source, 1_234_567);

    let mut options = options_with(OverwritePolicy::Always);
    options.preserve_timestamps = true;
    options.preserve_permissions = true;

    let mut engine = BufferedCopyEngine::new();
    engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .unwrap();

    assert_eq!(mtime_secs(&destination), 1_234_567);
}

#[tokio::test]
async fn timestamps_are_not_preserved_when_disabled() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "payload");
    set_mtime(&source, 1_234_567);

    let mut options = options_with(OverwritePolicy::Always);
    options.preserve_timestamps = false;
    options.preserve_permissions = false;

    let mut engine = BufferedCopyEngine::new();
    engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .unwrap();

    assert_ne!(
        mtime_secs(&destination),
        1_234_567,
        "the destination must keep its own timestamp"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unix_permission_bits_are_preserved() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.txt");
    let destination = dir.path().join("dest.txt");
    write_file(&source, "payload");
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o750)).unwrap();

    let mut options = options_with(OverwritePolicy::Always);
    options.preserve_permissions = true;

    let mut engine = BufferedCopyEngine::new();
    engine
        .copy_file_with_options(&source, &destination, options)
        .await
        .unwrap();

    let mode = std::fs::metadata(&destination)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o750);
}

#[test]
fn validate_rejects_prompt_without_handler() {
    let options = options_with(OverwritePolicy::Prompt);
    let error = options.validate().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
}

#[test]
fn validate_accepts_prompt_with_handler() {
    let mut options = options_with(OverwritePolicy::Prompt);
    options.overwrite_prompt = Some(OverwritePrompt::new(|_source, _destination| {
        OverwriteDecision::Skip
    }));
    options.validate().unwrap();
}
