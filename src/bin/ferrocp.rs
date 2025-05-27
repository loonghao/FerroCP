//! FerroCP command-line interface
//!
//! This binary provides a command-line interface to the FerroCP library,
//! offering high-performance cross-platform file copying functionality
//! implemented entirely in Rust with modern async I/O.

use clap::{Parser, Subcommand};
use ferrocp::{Config, EACopy};
use ferrocp::core::{ProgressInfo, FileOperations};
use std::path::PathBuf;
use std::time::Instant;
use tokio;
use tracing::{error, info, Level};
use tracing_subscriber;
use indicatif::{ProgressBar, ProgressStyle, HumanBytes, HumanDuration};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// FerroCP - High-performance cross-platform file copying tool
#[derive(Parser)]
#[command(name = "ferrocp")]
#[command(about = "High-performance cross-platform file copying tool written in Rust")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug output (more verbose than --verbose)
    #[arg(long)]
    debug: bool,

    /// Quiet mode - only show errors
    #[arg(short, long)]
    quiet: bool,

    /// Number of threads to use (0 = auto-detect CPU cores)
    #[arg(short, long, default_value = "0")]
    threads: usize,

    /// Compression level (0-22)
    #[arg(short, long, default_value = "3")]
    compression: i32,

    /// Buffer size in MB
    #[arg(short, long, default_value = "8")]
    buffer: usize,

    /// Disable progress reporting
    #[arg(long)]
    no_progress: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy a file or directory
    Copy {
        /// Source path
        source: PathBuf,
        /// Destination path
        destination: PathBuf,
        /// Preserve metadata
        #[arg(long)]
        preserve_metadata: bool,
        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,
        /// Follow symbolic links
        #[arg(long)]
        follow_symlinks: bool,
        /// Skip files that already exist and are newer or same size
        #[arg(long)]
        skip_existing: bool,
        /// Mirror mode - equivalent to robocopy /MIR (purge destination, skip existing)
        #[arg(long)]
        mirror: bool,
        /// Purge destination files that don't exist in source (used with --mirror)
        #[arg(long)]
        purge: bool,
    },
    /// Copy using network server acceleration
    Server {
        /// Source path
        source: PathBuf,
        /// Destination path
        destination: PathBuf,
        /// Server address
        server: String,
        /// Server port
        #[arg(short, long, default_value = "31337")]
        port: u16,
        /// Compression level for network transfer
        #[arg(short, long, default_value = "5")]
        compression: i32,
    },
    /// Perform delta copy using reference file
    Delta {
        /// Source path
        source: PathBuf,
        /// Destination path
        destination: PathBuf,
        /// Reference file path
        reference: PathBuf,
    },
    /// Start FerroCP service
    Service {
        /// Port to listen on
        #[arg(short, long, default_value = "31337")]
        port: u16,
        /// Number of worker threads
        #[arg(short, long, default_value = "4")]
        threads: usize,
    },
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging with proper level hierarchy
    let log_level = if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else if cli.quiet {
        Level::ERROR
    } else {
        Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Create configuration with default multi-threading
    let mut config = Config::new();

    // Set thread count - default to CPU cores if not specified
    let thread_count = if cli.threads > 0 {
        cli.threads
    } else {
        num_cpus::get().max(2) // At least 2 threads, up to CPU cores
    };

    config = config
        .with_thread_count(thread_count)
        .with_compression_level(cli.compression as u32)
        .with_buffer_size(cli.buffer * 1024 * 1024); // Convert MB to bytes

    // Execute command
    let result = match cli.command {
        Commands::Copy {
            source,
            destination,
            preserve_metadata,
            overwrite,
            follow_symlinks,
            skip_existing,
            mirror,
            purge,
        } => {
            copy_command(
                config,
                source,
                destination,
                preserve_metadata,
                overwrite,
                follow_symlinks,
                skip_existing,
                mirror,
                purge,
                !cli.no_progress,
                cli.quiet,
            ).await
        }
        Commands::Server {
            source,
            destination,
            server,
            port,
            compression,
        } => {
            server_copy_command(
                config.with_compression_level(compression as u32),
                source,
                destination,
                server,
                port,
                !cli.no_progress,
            ).await
        }
        Commands::Delta {
            source,
            destination,
            reference,
        } => {
            delta_copy_command(config, source, destination, reference, !cli.no_progress).await
        }
        Commands::Service { port, threads } => {
            service_command(port, threads).await
        }
        Commands::Version => {
            version_command().await
        }
    };

    if let Err(e) = result {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn copy_command(
    mut config: Config,
    source: PathBuf,
    destination: PathBuf,
    preserve_metadata: bool,
    overwrite: bool,
    follow_symlinks: bool,
    skip_existing: bool,
    mirror: bool,
    purge: bool,
    show_progress: bool,
    quiet: bool,
) -> ferrocp::Result<()> {
    // Handle mirror mode (equivalent to robocopy /MIR)
    let effective_skip_existing = skip_existing || mirror;
    let effective_purge = purge || mirror;

    config = config
        .with_preserve_metadata(preserve_metadata)
        .with_follow_symlinks(follow_symlinks)
        .with_skip_existing(effective_skip_existing);

    let mut eacopy = EACopy::with_config(config);

    // Setup progress bar if not quiet
    let progress_bar = if show_progress && !quiet {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏ ")
        );
        pb.set_message("Initializing...");
        Some(pb)
    } else {
        None
    };

    // Setup progress callback with modern progress bar
    if let Some(ref pb) = progress_bar {
        let pb_clone = pb.clone();
        eacopy = eacopy.with_progress_callback(move |progress: &ProgressInfo| {
            pb_clone.set_length(progress.current_total);
            pb_clone.set_position(progress.current_bytes);

            let file_name = progress.current_file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("...");

            pb_clone.set_message(format!("Copying {}", file_name));
        });
    }

    let start_time = Instant::now();

    // TODO: Implement purge functionality if effective_purge is true
    if effective_purge {
        // This would remove files in destination that don't exist in source
        // For now, just show a warning
        if !quiet {
            println!("Warning: Purge functionality not yet implemented");
        }
    }

    let stats = if tokio::fs::metadata(&source).await?.is_file() {
        eacopy.copy_file(&source, &destination).await?
    } else {
        eacopy.copy_directory(&source, &destination).await?
    };

    let duration = start_time.elapsed();

    // Finish progress bar
    if let Some(ref pb) = progress_bar {
        pb.finish_with_message("Copy completed");
    }

    // Always show completion summary unless quiet
    if !quiet {
        println!();
        println!("📋 Copy Summary:");
        println!("   Files copied: {}", stats.files_copied);
        if stats.files_skipped > 0 {
            println!("   Files skipped: {}", stats.files_skipped);
        }
        if stats.errors > 0 {
            println!("   Errors: {}", stats.errors);
        }
        println!("   Bytes copied: {}", HumanBytes(stats.bytes_copied));
        println!("   Duration: {}", HumanDuration(duration));

        if duration.as_secs_f64() > 0.0 && stats.bytes_copied > 0 {
            let speed_mb = (stats.bytes_copied as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();
            println!("   Speed: {:.2} MB/s", speed_mb);
        }

        if mirror {
            println!("   Mode: Mirror (equivalent to robocopy /MIR)");
        }
    }

    Ok(())
}

async fn server_copy_command(
    config: Config,
    source: PathBuf,
    destination: PathBuf,
    server: String,
    port: u16,
    show_progress: bool,
) -> ferrocp::Result<()> {
    let mut eacopy = EACopy::with_config(config);

    if show_progress {
        eacopy = eacopy.with_progress_callback(|progress: &ProgressInfo| {
            let percent = if progress.total_size > 0 {
                (progress.total_bytes as f64 / progress.total_size as f64) * 100.0
            } else {
                0.0
            };

            let speed_mb = progress.speed / (1024.0 * 1024.0);
            print!("\rServer copy: {:.1}% ({:.2} MB/s)", percent, speed_mb);
        });
    }

    let start_time = Instant::now();
    let stats = eacopy.copy_with_server(&source, &destination, &server, port).await?;

    if show_progress {
        println!(); // New line after progress
    }

    let duration = start_time.elapsed();
    info!(
        "Server copy completed: {} files, {} bytes in {:.2}s",
        stats.files_copied,
        stats.bytes_copied,
        duration.as_secs_f64()
    );

    Ok(())
}

async fn delta_copy_command(
    config: Config,
    source: PathBuf,
    destination: PathBuf,
    reference: PathBuf,
    show_progress: bool,
) -> ferrocp::Result<()> {
    let mut eacopy = EACopy::with_config(config);

    if show_progress {
        eacopy = eacopy.with_progress_callback(|progress: &ProgressInfo| {
            let percent = if progress.current_total > 0 {
                (progress.current_bytes as f64 / progress.current_total as f64) * 100.0
            } else {
                0.0
            };

            print!("\rDelta copy: {:.1}%", percent);
        });
    }

    let start_time = Instant::now();
    let stats = eacopy.delta_copy(&source, &destination, &reference).await?;

    if show_progress {
        println!(); // New line after progress
    }

    let duration = start_time.elapsed();
    info!(
        "Delta copy completed: {} bytes in {:.2}s",
        stats.bytes_copied,
        duration.as_secs_f64()
    );

    Ok(())
}

async fn service_command(port: u16, threads: usize) -> ferrocp::Result<()> {
    info!("Starting FerroCP service on port {} with {} threads", port, threads);

    // TODO: Implement actual service
    // For now, just print a message
    println!("FerroCP service would start here (not yet implemented)");

    Ok(())
}

async fn version_command() -> ferrocp::Result<()> {
    println!("ferrocp {}", env!("CARGO_PKG_VERSION"));
    println!("A high-performance cross-platform file copying tool written in Rust");
    println!("FerroCP - Fast, reliable, and efficient file operations");
    Ok(())
}
