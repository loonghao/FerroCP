//! Enhanced display utilities for FerroCP CLI

use console::style;
use ferrocp_device::{DeviceComparison, DeviceInfo};
use ferrocp_types::{CopyStats, DeviceType};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Display device information in a formatted way
pub fn display_device_info(label: &str, device_info: &DeviceInfo) {
    println!();
    println!(
        "{} {}",
        style("📀").blue().bold(),
        style(label).bold().underlined()
    );
    println!(
        "  Type: {}",
        style(device_info.device_type_description()).cyan()
    );
    println!("  Filesystem: {}", style(&device_info.filesystem).cyan());
    println!("  Space: {}", style(device_info.format_space_info()).cyan());
    println!(
        "  Read Speed: {} MB/s (theoretical)",
        style(format!("{:.0}", device_info.theoretical_read_speed_mbps())).green()
    );
    println!(
        "  Write Speed: {} MB/s (theoretical)",
        style(format!("{:.0}", device_info.theoretical_write_speed_mbps())).green()
    );
    println!(
        "  Optimal Buffer: {}",
        style(device_info.format_buffer_size()).yellow()
    );
}

/// Display device comparison and performance expectations
pub fn display_device_comparison(comparison: &DeviceComparison) {
    println!();
    println!(
        "{} {}",
        style("⚡").yellow().bold(),
        style("Performance Analysis").bold().underlined()
    );
    println!(
        "  Expected Speed: {} MB/s",
        style(format!("{:.0}", comparison.expected_speed_mbps))
            .green()
            .bold()
    );
    println!(
        "  Bottleneck: {}",
        style(comparison.bottleneck_description()).yellow()
    );

    let recommendations = comparison.get_recommendations();
    if !recommendations.is_empty() {
        println!("  Recommendations:");
        for rec in recommendations {
            println!("    • {}", style(rec).dim());
        }
    }
}

/// Create an enhanced progress bar for file operations
pub fn create_progress_bar(quiet: bool) -> Option<ProgressBar> {
    if quiet {
        return None;
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg} {wide_bar:.cyan/blue} {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    Some(pb)
}

/// Update progress bar with current file information
pub fn update_progress_with_file_info(
    pb: &ProgressBar,
    current_file: &str,
    files_processed: u64,
    total_files: u64,
) {
    let filename = std::path::Path::new(current_file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(current_file);

    pb.set_position(files_processed);
    pb.set_length(total_files);
    pb.set_message(format!("Copying: {}", filename));
}

/// Display enhanced copy statistics with performance analysis
pub fn display_enhanced_copy_stats(stats: &CopyStats, comparison: Option<&DeviceComparison>) {
    println!();
    println!("{}", style("Copy Statistics:").bold().underlined());

    // Basic statistics
    println!("  Files copied: {}", style(stats.files_copied).green());
    println!(
        "  Directories created: {}",
        style(stats.directories_created).green()
    );
    println!(
        "  Bytes copied: {}",
        style(format_bytes(stats.bytes_copied)).green()
    );
    println!("  Files skipped: {}", style(stats.files_skipped).yellow());

    // Error handling
    println!(
        "  Errors: {}",
        if stats.errors > 0 {
            style(stats.errors).red()
        } else {
            style(stats.errors).green()
        }
    );

    // Performance metrics
    println!(
        "  Duration: {}",
        style(format_duration(stats.duration)).blue()
    );

    let actual_speed = stats.transfer_rate() / 1024.0 / 1024.0;
    println!(
        "  Transfer rate: {} MB/s",
        style(format!("{:.2}", actual_speed)).blue().bold()
    );

    // Performance comparison
    if let Some(comparison) = comparison {
        let efficiency = (actual_speed / comparison.expected_speed_mbps) * 100.0;
        let efficiency_color = if efficiency >= 80.0 {
            console::Color::Green
        } else if efficiency >= 60.0 {
            console::Color::Yellow
        } else {
            console::Color::Red
        };

        println!(
            "  Expected Speed: {} MB/s",
            style(format!("{:.0}", comparison.expected_speed_mbps)).dim()
        );
        let efficiency_style = match efficiency_color {
            console::Color::Green => style(format!("{:.1}", efficiency)).green().bold(),
            console::Color::Yellow => style(format!("{:.1}", efficiency)).yellow().bold(),
            console::Color::Red => style(format!("{:.1}", efficiency)).red().bold(),
            _ => style(format!("{:.1}", efficiency)).bold(),
        };

        println!("  Efficiency: {}%", efficiency_style);
    }

    // Zero-copy statistics
    println!(
        "  Zero-copy operations: {}",
        style(stats.zerocopy_operations).cyan()
    );
    println!(
        "  Zero-copy efficiency: {:.1}%",
        style(stats.zerocopy_efficiency() * 100.0).cyan()
    );
}

/// Format bytes in human-readable format
pub fn format_bytes(bytes: u64) -> String {
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
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{:.2}s", duration.as_secs_f64())
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

/// Display a warning message with proper formatting
pub fn display_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
}

/// Display an error message with proper formatting
pub fn display_error(message: &str) {
    println!("{} {}", style("✗").red().bold(), style(message).red());
}

/// Display a success message with proper formatting
pub fn display_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), style(message).green());
}

/// Display an info message with proper formatting
pub fn display_info(message: &str) {
    println!("{} {}", style("ℹ").blue().bold(), style(message).blue());
}

/// Create a spinner for analysis phase
pub fn create_analysis_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Display device type with appropriate icon
pub fn device_type_icon(device_type: DeviceType) -> &'static str {
    match device_type {
        DeviceType::SSD => "💾",
        DeviceType::HDD => "💿",
        DeviceType::Network => "🌐",
        DeviceType::RamDisk => "⚡",
        DeviceType::Unknown => "❓",
    }
}
