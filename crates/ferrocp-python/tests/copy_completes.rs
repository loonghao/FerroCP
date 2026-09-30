//! Regression test for the blocker that used to keep the Python API from
//! performing a real copy.
//!
//! `ferrocp.copy_file(...)` / `copy_directory(...)` resolve to
//! `PyCopyEngine::copy_file`, which drives `CopyEngine::execute()`. That call
//! submits the task to the scheduler and then waits for completion, but the
//! dispatch loop that hands the task to the executor used to live only in
//! `CopyEngine::start()` and was never run, so the submitted task stayed
//! `Pending` in the scheduler. The await then only resolved when the
//! executor's 3600 second timeout fired, surfacing as
//! `Err(Timeout waiting for task ...)` after an hour.
//!
//! `CopyEngine::execute()` now starts the dispatch loop on demand, so this
//! test runs again. The 15 second timeout is what fails if the loop ever stops
//! being started: a healthy copy finishes in milliseconds.

use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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
