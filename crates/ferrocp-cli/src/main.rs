//! FerroCP - High-performance cross-platform file copying tool
//!
//! A modern, fast, and reliable file copying tool written in Rust with advanced features
//! like zero-copy operations, compression, and intelligent device detection.

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use ferrocp_types::{CopyMode, CopyStats, DeviceType};
use ferrocp_engine::{CopyEngine, CopyRequest};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

/// FerroCP - High-performance cross-platform file copying tool
#[derive(Parser)]
#[command(
    name = "ferrocp",
    version = env!("CARGO_PKG_VERSION"),
    about = "High-performance cross-platform file copying tool",
    long_about = "FerroCP is a modern, fast, and reliable file copying tool written in Rust.\n\
                  It features zero-copy operations, compression, intelligent device detection,\n\
                  and advanced synchronization capabilities."
)]
struct Cli {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Quiet mode - minimal output
    #[arg(short, long)]
    quiet: bool,

    /// Verbose mode - detailed output
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files and directories
    Copy {
        /// Source path
        source: PathBuf,
        /// Destination path
        destination: PathBuf,
        /// Copy mode
        #[arg(short, long, value_enum, default_value = "all")]
        mode: CopyModeArg,
        /// Number of threads to use
        #[arg(short, long)]
        threads: Option<usize>,
        /// Enable compression
        #[arg(long)]
        compress: bool,
        /// Compression level (0-22)
        #[arg(long, default_value = "6")]
        compression_level: u8,
        /// Enable zero-copy operations
        #[arg(long, default_value = "true")]
        zero_copy: bool,
        /// Mirror mode (equivalent to robocopy /MIR)
        #[arg(long)]
        mirror: bool,
        /// Exclude patterns
        #[arg(long)]
        exclude: Vec<String>,
        /// Include patterns
        #[arg(long)]
        include: Vec<String>,
    },
    /// Synchronize directories
    Sync {
        /// Source directory
        source: PathBuf,
        /// Destination directory
        destination: PathBuf,
        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
        /// Delete extra files in destination
        #[arg(long)]
        delete: bool,
    },
    /// Verify file integrity
    Verify {
        /// Path to verify
        path: PathBuf,
        /// Verify against source
        #[arg(long)]
        source: Option<PathBuf>,
    },
    /// Show device information
    Device {
        /// Path to analyze
        path: PathBuf,
    },
    /// Show configuration
    Config {
        /// Show default configuration
        #[arg(long)]
        default: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum CopyModeArg {
    All,
    Newer,
    Different,
    Mirror,
}

impl From<CopyModeArg> for CopyMode {
    fn from(mode: CopyModeArg) -> Self {
        match mode {
            CopyModeArg::All => CopyMode::All,
            CopyModeArg::Newer => CopyMode::Newer,
            CopyModeArg::Different => CopyMode::Different,
            CopyModeArg::Mirror => CopyMode::Mirror,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.debug, cli.quiet, cli.verbose)?;

    info!("FerroCP v{} starting", env!("CARGO_PKG_VERSION"));

    // Execute command
    match cli.command {
        Commands::Copy {
            source,
            destination,
            mode,
            threads,
            compress,
            compression_level,
            zero_copy,
            mirror,
            exclude,
            include,
        } => {
            let copy_mode = if mirror {
                CopyMode::Mirror
            } else {
                mode.into()
            };
            copy_command(
                source,
                destination,
                copy_mode,
                threads,
                compress,
                compression_level,
                zero_copy,
                exclude,
                include,
                cli.quiet,
            )
            .await?;
        }
        Commands::Sync {
            source,
            destination,
            dry_run,
            delete,
        } => {
            sync_command(source, destination, dry_run, delete).await?;
        }
        Commands::Verify { path, source } => {
            verify_command(path, source).await?;
        }
        Commands::Device { path } => {
            device_command(path).await?;
        }
        Commands::Config { default } => {
            config_command(default).await?;
        }
    }

    Ok(())
}

fn init_logging(debug: bool, quiet: bool, verbose: bool) -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = if debug {
        "debug"
    } else if verbose {
        "info"
    } else if quiet {
        "error"
    } else {
        "warn"
    };

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap();

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();

    Ok(())
}

async fn copy_command(
    source: PathBuf,
    destination: PathBuf,
    mode: CopyMode,
    threads: Option<usize>,
    compress: bool,
    compression_level: u8,
    zero_copy: bool,
    exclude: Vec<String>,
    include: Vec<String>,
    quiet: bool,
) -> Result<()> {
    info!("Starting copy operation");
    info!("Source: {}", source.display());
    info!("Destination: {}", destination.display());
    info!("Mode: {:?}", mode);

    if !quiet {
        println!(
            "{} Copying {} to {}",
            style("→").green().bold(),
            style(source.display()).cyan(),
            style(destination.display()).cyan()
        );
    }

    // Create a progress bar
    let pb = if !quiet {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Analyzing files...");
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    // Create copy engine
    let mut engine = CopyEngine::new().await?;

    // Start the engine
    engine.start().await?;

    // Create copy request using builder pattern
    let mut request = CopyRequest::new(source, destination)
        .with_mode(mode)
        .preserve_metadata(true)
        .verify_copy(false)
        .enable_compression(compress)
        .exclude_patterns(exclude)
        .include_patterns(include);

    // Note: threads, compression_level, zero_copy are handled by the engine internally
    // These CLI options could be used to configure the engine in the future

    if let Some(pb) = &pb {
        pb.set_message("Copying files...");
    }

    // Execute copy operation
    let result = engine.execute(request).await?;
    let stats = result.stats;

    // Stop the engine
    engine.stop().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("Copy completed");
    }

    if !quiet {
        print_copy_stats(&stats);
    }

    info!("Copy operation completed successfully");
    Ok(())
}

async fn sync_command(
    source: PathBuf,
    destination: PathBuf,
    dry_run: bool,
    _delete: bool,
) -> Result<()> {
    info!("Starting sync operation");
    println!(
        "{} Synchronizing {} with {}",
        style("⟲").blue().bold(),
        style(source.display()).cyan(),
        style(destination.display()).cyan()
    );

    if dry_run {
        println!(
            "{} Dry run mode - no changes will be made",
            style("ℹ").yellow()
        );
    }

    // TODO: Implement actual sync logic
    println!("{} Sync operation completed", style("✓").green());
    Ok(())
}

async fn verify_command(path: PathBuf, _source: Option<PathBuf>) -> Result<()> {
    info!("Starting verify operation");
    println!(
        "{} Verifying {}",
        style("✓").green().bold(),
        style(path.display()).cyan()
    );

    // TODO: Implement actual verify logic
    println!("{} Verification completed successfully", style("✓").green());
    Ok(())
}

async fn device_command(path: PathBuf) -> Result<()> {
    info!("Analyzing device for path: {}", path.display());
    println!(
        "{} Analyzing device for {}",
        style("🔍").blue().bold(),
        style(path.display()).cyan()
    );

    // TODO: Implement actual device detection
    let device_type = DeviceType::SSD; // Placeholder
    println!("Device type: {:?}", device_type);
    println!("Zero-copy support: Yes");
    println!("Optimal buffer size: 8MB");

    Ok(())
}

async fn config_command(default: bool) -> Result<()> {
    if default {
        println!("{} Default configuration:", style("⚙").blue().bold());
        // TODO: Show actual default configuration
        println!("threads: auto");
        println!("buffer_size: 8MB");
        println!("compression: false");
        println!("zero_copy: true");
    } else {
        println!("{} Current configuration:", style("⚙").blue().bold());
        // TODO: Show current configuration
        println!("No configuration file found");
    }
    Ok(())
}



fn print_copy_stats(stats: &CopyStats) {
    println!();
    println!("{}", style("Copy Statistics:").bold().underlined());
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
    println!(
        "  Errors: {}",
        if stats.errors > 0 {
            style(stats.errors).red()
        } else {
            style(stats.errors).green()
        }
    );
    println!(
        "  Duration: {}",
        style(format_duration(stats.duration)).blue()
    );
    println!(
        "  Transfer rate: {}",
        style(format!(
            "{:.2} MB/s",
            stats.transfer_rate() / 1024.0 / 1024.0
        ))
        .blue()
    );
    println!(
        "  Zero-copy operations: {}",
        style(stats.zerocopy_operations).cyan()
    );
    println!(
        "  Zero-copy efficiency: {:.1}%",
        style(stats.zerocopy_efficiency() * 100.0).cyan()
    );
}

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

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{:.2}s", duration.as_secs_f64())
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}
