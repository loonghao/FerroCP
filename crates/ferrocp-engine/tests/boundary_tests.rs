//! Boundary tests for directory trees
//!
//! `ferrocp-io` owns the file-level copy, so its boundary tests cover single
//! files. Directory trees are walked here, in `ferrocp-engine`, which is the
//! crate that decides what to do with each entry it finds.
//!
//! These run through the public [`CopyEngine`] API rather than the executor's
//! internals, so they exercise the same path a CLI invocation or a Python call
//! takes. `CopyEngine::execute` starts the dispatch loop on demand, so no
//! `start()` call is needed.

use ferrocp_engine::{task::CopyRequest, CopyEngine};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Copy `source` to `destination` and fail the test with a useful message
///
/// The 60 second envelope matters: a tree copy that is never dispatched does
/// not fail, it waits for the executor's one hour timeout. Bounding the wait
/// turns that hang into a reportable failure.
async fn copy_tree(source: &Path, destination: &Path) -> ferrocp_engine::task::CopyResult {
    let engine = CopyEngine::new().await.expect("engine construction failed");
    let request = CopyRequest::new(source, destination);

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(60), engine.execute(request))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "copying {} to {} did not finish in 60s",
                source.display(),
                destination.display()
            )
        });

    let result = outcome.expect("copy returned an error");
    assert!(
        result.is_success(),
        "copying {} to {} failed: {:?}",
        source.display(),
        destination.display(),
        result.error
    );
    result
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    file.sync_all().unwrap();
}

/// Build `depth` nested directories below `root` and return the deepest one
///
/// Components are two characters long on purpose. The depth is what is under
/// test, and short components keep the absolute path inside the 260 character
/// `MAX_PATH` limit that still applies to Win32 paths on a CI runner, whatever
/// temporary directory the test is handed.
fn nest_directories(root: &Path, depth: usize) -> PathBuf {
    let mut current = root.to_path_buf();
    for level in 0..depth {
        current = current.join(format!("{level:02}"));
    }
    std::fs::create_dir_all(&current).unwrap();
    current
}

#[tokio::test]
async fn deeply_nested_tree_is_reproduced_in_full() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let destination_root = dir.path().join("destination");

    let deepest = nest_directories(&source_root, 32);
    write_file(&deepest.join("leaf.txt"), "leaf payload");
    write_file(&source_root.join("root.txt"), "root payload");

    let stats = copy_tree(&source_root, &destination_root).await;

    assert_eq!(
        std::fs::read_to_string(destination_root.join("root.txt")).unwrap(),
        "root payload"
    );

    let relative_leaf = deepest.strip_prefix(&source_root).unwrap().join("leaf.txt");
    let copied_leaf = destination_root.join(&relative_leaf);
    assert_eq!(
        std::fs::read_to_string(&copied_leaf).unwrap(),
        "leaf payload",
        "the deep branch was not reproduced at {}",
        copied_leaf.display()
    );
    assert_eq!(stats.stats.errors, 0, "stats were: {:?}", stats.stats);
}

/// A tree whose every name is awkward: non-ASCII directories, spaces, and
/// characters that need escaping.
#[tokio::test]
async fn awkward_names_survive_a_tree_copy() {
    // No double quotes in these names: Windows rejects them outright, and the
    // point of the test is how FerroCP handles names the filesystem accepts.
    let directories = ["源 directory", "nested with spaces", "файлы"];
    let files = [
        "plain with spaces.txt",
        "café-ünïcode.txt",
        "中文文件.txt",
        "emoji-\u{1f4c1}-file.txt",
        "apostrophe's and [brackets].txt",
    ];

    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source tree");
    let destination_root = dir.path().join("destination tree");

    for (dir_index, directory) in directories.iter().enumerate() {
        for (file_index, file) in files.iter().enumerate() {
            let name = format!("{directory}/{dir_index}-{file_index}-{file}");
            write_file(&source_root.join(&name), &format!("payload {name}"));
        }
    }

    let stats = copy_tree(&source_root, &destination_root).await;

    for (dir_index, directory) in directories.iter().enumerate() {
        for (file_index, file) in files.iter().enumerate() {
            let name = format!("{directory}/{dir_index}-{file_index}-{file}");
            let copied = destination_root.join(&name);
            assert!(
                copied.exists(),
                "{name:?} was not created at {}",
                copied.display()
            );
            assert_eq!(
                std::fs::read_to_string(&copied).unwrap(),
                format!("payload {name}")
            );
        }
    }
    assert_eq!(stats.stats.errors, 0, "stats were: {:?}", stats.stats);
}

/// An empty source directory produces an empty destination directory, not an
/// error and not a missing destination.
#[tokio::test]
async fn empty_source_directory_produces_an_empty_destination() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("empty-source");
    let destination_root = dir.path().join("empty-destination");
    std::fs::create_dir_all(&source_root).unwrap();

    copy_tree(&source_root, &destination_root).await;

    assert!(
        destination_root.is_dir(),
        "the destination directory must be created"
    );
    assert_eq!(
        std::fs::read_dir(&destination_root).unwrap().count(),
        0,
        "an empty source must not gain entries"
    );
}

/// A tree made only of nested empty directories is reproduced, not collapsed.
#[tokio::test]
async fn tree_of_empty_directories_is_reproduced() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    let destination_root = dir.path().join("destination");
    nest_directories(&source_root, 8);

    copy_tree(&source_root, &destination_root).await;

    let relative = nest_directories(&Path::new(""), 8);
    let copied_deepest = destination_root.join(relative);
    assert!(
        copied_deepest.is_dir(),
        "the empty branch was not reproduced at {}",
        copied_deepest.display()
    );
}
