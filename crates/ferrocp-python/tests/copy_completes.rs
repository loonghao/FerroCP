//! Regression test for the blocker that keeps the Python API from performing a
//! real copy.
//!
//! `ferrocp.copy_file(...)` / `copy_directory(...)` resolve to
//! `PyCopyEngine::copy_file`, which drives `CopyEngine::execute()`. That call
//! submits the task to the scheduler and then waits for completion, but the
//! dispatch loop that would hand the task to the executor lives in
//! `CopyEngine::start()` and is never run, so the submitted task stays
//! `Pending` in the scheduler and is never executed. The await then only
//! resolves when the executor's 3600 second timeout fires, surfacing as
//! `Err(Timeout waiting for task ...)` after an hour.
//!
//! The defect is in `ferrocp-engine` (`scheduler.rs` / `engine.rs`), not in the
//! Python bindings, so this test is ignored here and only documents the
//! blocker. Re-enable it once the engine dispatch path is fixed.

use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "blocker: a copy submitted through the Python bindings is never executed because the scheduler dispatch loop is not started, so the await only resolves when the executor's 3600 second timeout fires"]
async fn engine_execute_completes() {
    let dir = std::env::temp_dir().join("ferrocp_engine_execute_completes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.txt");
    let dst = dir.join("dst.txt");
    std::fs::write(&src, b"payload".repeat(1000)).unwrap();

    let engine = ferrocp_engine::CopyEngine::new().await.unwrap();
    let req = ferrocp_engine::task::CopyRequest::new(src, dst.clone());

    let outcome = tokio::time::timeout(Duration::from_secs(15), engine.execute(req)).await;
    assert!(outcome.is_ok(), "CopyEngine::execute deadlocked");

    let result = outcome.unwrap().expect("copy failed");
    assert!(dst.exists(), "destination file was not created");
    assert_eq!(
        std::fs::read(&dst).unwrap().len(),
        result.stats.bytes_copied as usize
    );
}
