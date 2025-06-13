//! Enhanced progress tracking for CLI

use console::style;
use ferrocp_types::ProgressInfo;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Enhanced progress tracker with file-level information
pub struct EnhancedProgressTracker {
    progress_bar: Option<ProgressBar>,
    last_update: Arc<RwLock<Instant>>,
    update_interval: Duration,
    quiet: bool,
}

impl EnhancedProgressTracker {
    /// Create a new enhanced progress tracker
    pub fn new(quiet: bool) -> Self {
        let progress_bar = if !quiet {
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} {msg} [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  "),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        Self {
            progress_bar,
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval: Duration::from_millis(200), // Update every 200ms
            quiet,
        }
    }

    /// Update progress with current file information
    pub async fn update_progress(&self, progress: &ProgressInfo) {
        // Check if enough time has passed since last update
        {
            let mut last_update = self.last_update.write().await;
            if last_update.elapsed() < self.update_interval {
                return;
            }
            *last_update = Instant::now();
        }

        if let Some(pb) = &self.progress_bar {
            // Extract filename from path
            let filename = progress
                .current_file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Calculate progress percentage
            let file_progress = if progress.current_file_size > 0 {
                (progress.current_file_bytes as f64 / progress.current_file_size as f64 * 100.0)
                    as u64
            } else {
                0
            };

            let overall_progress = if progress.total_bytes > 0 {
                (progress.bytes_processed as f64 / progress.total_bytes as f64 * 100.0) as u64
            } else {
                0
            };

            // Format transfer rate
            let rate_mbps = progress.transfer_rate / 1024.0 / 1024.0;

            // Format ETA
            let eta_str = if let Some(eta) = progress.eta {
                format_duration(eta)
            } else {
                "unknown".to_string()
            };

            // Update progress bar
            pb.set_position(overall_progress);
            pb.set_length(100);

            // Create detailed message
            let message = format!(
                "Copying: {} ({:.1}%) - {:.1} MB/s - ETA: {}",
                filename, file_progress, rate_mbps, eta_str
            );
            pb.set_message(message);

            // Show file-level progress in quiet intervals
            if !self.quiet && progress.files_processed % 100 == 0 {
                self.display_file_info(progress).await;
            }
        } else if !self.quiet {
            // Fallback for when progress bar is not available
            self.display_simple_progress(progress).await;
        }
    }

    /// Display detailed file information
    async fn display_file_info(&self, progress: &ProgressInfo) {
        let filename = progress
            .current_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let file_size = format_bytes(progress.current_file_size);
        let rate_mbps = progress.transfer_rate / 1024.0 / 1024.0;

        println!(
            "  {} {} ({}) - {:.1} MB/s",
            style("📄").blue(),
            style(filename).cyan(),
            style(file_size).dim(),
            style(format!("{:.1}", rate_mbps)).green()
        );
    }

    /// Display simple progress without progress bar
    async fn display_simple_progress(&self, progress: &ProgressInfo) {
        let filename = progress
            .current_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let overall_progress = if progress.total_bytes > 0 {
            progress.bytes_processed as f64 / progress.total_bytes as f64 * 100.0
        } else {
            0.0
        };

        let rate_mbps = progress.transfer_rate / 1024.0 / 1024.0;

        println!(
            "\r{} Copying: {} ({:.1}%) - {:.1} MB/s",
            style("→").green(),
            style(filename).cyan(),
            style(format!("{:.1}", overall_progress)).yellow(),
            style(format!("{:.1}", rate_mbps)).green()
        );
    }

    /// Finish the progress tracking
    pub fn finish(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message(message.to_string());
        }
    }

    /// Finish and clear the progress bar
    pub fn finish_and_clear(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_and_clear();
        }
    }

    /// Set a message without updating progress
    pub fn set_message(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.set_message(message.to_string());
        }
    }

    /// Display a warning during progress
    pub fn display_warning(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.suspend(|| {
                println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
            });
        } else {
            println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
        }
    }

    /// Display an error during progress
    pub fn display_error(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.suspend(|| {
                println!("{} {}", style("✗").red().bold(), style(message).red());
            });
        } else {
            println!("{} {}", style("✗").red().bold(), style(message).red());
        }
    }

    /// Display file completion
    pub async fn display_file_completed(&self, filename: &str, bytes: u64, duration: Duration) {
        if self.quiet {
            return;
        }

        let rate_mbps = if duration.as_secs_f64() > 0.0 {
            bytes as f64 / duration.as_secs_f64() / 1024.0 / 1024.0
        } else {
            0.0
        };

        let message = format!(
            "✓ {} ({}) - {:.1} MB/s",
            filename,
            format_bytes(bytes),
            rate_mbps
        );

        if let Some(pb) = &self.progress_bar {
            pb.suspend(|| {
                println!("  {}", style(message).green().dim());
            });
        } else {
            println!("  {}", style(message).green().dim());
        }
    }
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Format duration in human-readable format
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{:.0}s", duration.as_secs_f64())
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

/// Create a progress callback for the copy engine
pub fn create_progress_callback(
    tracker: Arc<EnhancedProgressTracker>,
) -> impl Fn(ProgressInfo) + Send + Sync {
    move |progress: ProgressInfo| {
        let tracker = Arc::clone(&tracker);
        tokio::spawn(async move {
            tracker.update_progress(&progress).await;
        });
    }
}
