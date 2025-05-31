# Skip Existing Files Performance Optimization

## Current Performance Issues

Based on analysis of the current implementation in `src/core.rs`, several performance bottlenecks have been identified when skipping existing files:

### 1. Sequential Metadata Checks
- **Problem**: Each file's metadata is checked individually using `fs::metadata()` calls
- **Impact**: High I/O overhead, especially for directories with many files
- **Current Code**: Lines 700-701 in `should_skip_file()`

### 2. Redundant File Existence Checks
- **Problem**: `exists()` check followed by `metadata()` call duplicates filesystem access
- **Impact**: Double I/O operations per file
- **Current Code**: Lines 696 and 700-701

### 3. No Batch Processing for Skip Checks
- **Problem**: Files are processed one by one without batching metadata operations
- **Impact**: Poor performance on directories with thousands of small files

### 4. Inefficient Directory Traversal
- **Problem**: Skip checks happen during file copy, not during directory traversal
- **Impact**: Unnecessary processing overhead

## Robocopy Performance Comparison

Robocopy is faster at skipping files because:

1. **Batch Metadata Operations**: Reads directory metadata in chunks
2. **Optimized File System Calls**: Uses Windows-specific APIs for faster metadata access
3. **Early Skip Detection**: Filters files during directory enumeration
4. **Minimal Logging**: Reduces overhead when skipping files

## Optimization Strategies

### 1. Batch Metadata Collection

```rust
// Proposed optimization: Collect metadata for multiple files at once
async fn collect_batch_metadata(paths: &[PathBuf]) -> Result<Vec<Option<Metadata>>> {
    let tasks: Vec<_> = paths.iter()
        .map(|path| async move { fs::metadata(path).await.ok() })
        .collect();
    
    Ok(futures::future::join_all(tasks).await)
}
```

### 2. Combined Existence and Metadata Check

```rust
// Optimize: Single filesystem call instead of two
async fn get_metadata_if_exists<P: AsRef<Path>>(path: P) -> Option<Metadata> {
    fs::metadata(path).await.ok()
}
```

### 3. Early Skip Detection During Directory Traversal

```rust
// Filter files during directory walk, not during copy
async fn collect_files_to_copy(
    source_dir: &Path,
    dest_dir: &Path,
    skip_existing: bool,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files_to_copy = Vec::new();
    
    for entry in WalkDir::new(source_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let src_path = entry.path();
            let rel_path = src_path.strip_prefix(source_dir)?;
            let dest_path = dest_dir.join(rel_path);
            
            if skip_existing {
                if should_skip_file_fast(src_path, &dest_path).await? {
                    continue; // Skip this file
                }
            }
            
            files_to_copy.push((src_path.to_path_buf(), dest_path));
        }
    }
    
    Ok(files_to_copy)
}
```

### 4. Optimized Skip Logic

```rust
// Fast skip check with minimal I/O
async fn should_skip_file_fast(source: &Path, destination: &Path) -> Result<bool> {
    // Single metadata call for destination
    let dest_metadata = match fs::metadata(destination).await {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false), // Destination doesn't exist, don't skip
    };
    
    // Only get source metadata if destination exists
    let source_metadata = fs::metadata(source).await?;
    
    // Quick size check first (fastest comparison)
    if dest_metadata.len() != source_metadata.len() {
        return Ok(false);
    }
    
    // Then check modification time
    let source_modified = source_metadata.modified().unwrap_or(UNIX_EPOCH);
    let dest_modified = dest_metadata.modified().unwrap_or(UNIX_EPOCH);
    
    Ok(dest_modified >= source_modified)
}
```

### 5. Parallel Skip Checking

```rust
// Process skip checks in parallel for better performance
async fn batch_skip_check(
    file_pairs: Vec<(PathBuf, PathBuf)>,
    max_concurrent: usize,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    use futures::stream::{self, StreamExt};
    
    let files_to_copy: Vec<_> = stream::iter(file_pairs)
        .map(|(src, dest)| async move {
            if should_skip_file_fast(&src, &dest).await.unwrap_or(false) {
                None // Skip this file
            } else {
                Some((src, dest)) // Copy this file
            }
        })
        .buffer_unordered(max_concurrent)
        .filter_map(|result| async move { result })
        .collect()
        .await;
    
    Ok(files_to_copy)
}
```

## Implementation Plan

### Phase 1: Core Optimizations
1. Replace dual `exists()` + `metadata()` calls with single `metadata()` call
2. Implement fast skip logic with size check first
3. Add batch metadata collection for small files

### Phase 2: Directory Processing Optimization
1. Move skip detection to directory traversal phase
2. Implement parallel skip checking
3. Add progress reporting for skip operations

### Phase 3: Advanced Optimizations
1. Implement platform-specific optimizations (Windows FindFirstFile, Linux readdir)
2. Add caching for frequently accessed directory metadata
3. Optimize for different file size distributions

## Expected Performance Improvements

Based on similar optimizations in other tools:

- **Small files (< 1MB)**: 3-5x faster skip detection
- **Large directories (1000+ files)**: 5-10x faster overall processing
- **Mixed workloads**: 2-3x improvement in skip-heavy scenarios

## Benchmarking Strategy

1. **Baseline Measurement**: Current implementation vs robocopy
2. **Incremental Testing**: Each optimization phase measured separately
3. **Regression Testing**: Ensure optimizations don't break correctness
4. **Real-world Scenarios**: Test with actual project directories

## Configuration Options

Add new configuration options for skip optimization:

```rust
pub struct SkipOptimizationConfig {
    /// Maximum number of concurrent skip checks
    pub max_concurrent_skip_checks: usize,
    /// Batch size for metadata collection
    pub metadata_batch_size: usize,
    /// Enable fast skip mode (size + mtime only)
    pub fast_skip_mode: bool,
    /// Cache directory metadata for repeated operations
    pub cache_directory_metadata: bool,
}
```

This optimization plan addresses the core performance issues while maintaining compatibility and correctness.
