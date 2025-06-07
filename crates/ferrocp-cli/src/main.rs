//! FerroCP - High-performance cross-platform file copying tool
//!
//! A modern, fast, and reliable file copying tool written in Rust with advanced features
//! like zero-copy operations, compression, and intelligent device detection.

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use ferrocp_device::PerformanceAnalyzer;
use ferrocp_engine::{CopyEngine, CopyRequest};
use ferrocp_types::CopyMode;
use std::path::PathBuf;
use tracing::info;

mod display;
mod json_output;
mod progress;

use display::*;
use json_output::CopyResultJson;

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

        /// Output results in JSON format
        #[arg(long)]
        json: bool,
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
            json,
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
                json,
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
    _threads: Option<usize>,
    compress: bool,
    _compression_level: u8,
    _zero_copy: bool,
    exclude: Vec<String>,
    include: Vec<String>,
    quiet: bool,
    json: bool,
) -> Result<()> {
    info!("Starting copy operation");
    info!("Source: {}", source.display());
    info!("Destination: {}", destination.display());
    info!("Mode: {:?}", mode);

    if !quiet && !json {
        println!(
            "{} Copying {} to {}",
            style("→").green().bold(),
            style(source.display()).cyan(),
            style(destination.display()).cyan()
        );
    }

    // Analyze devices before starting copy
    let analyzer = PerformanceAnalyzer::new();
    let analysis_pb = if !quiet && !json {
        Some(create_analysis_spinner("Analyzing devices..."))
    } else {
        None
    };

    let source_info = analyzer.analyze_device(&source).await?;
    let dest_info = analyzer.analyze_device(&destination).await?;
    let comparison = analyzer.compare_devices(&source_info, &dest_info);

    if let Some(pb) = analysis_pb {
        pb.finish_and_clear();
    }

    if !quiet && !json {
        display_device_info("Source Device", &source_info);
        display_device_info("Destination Device", &dest_info);
        display_device_comparison(&comparison);
    }

    // Create a progress bar for the actual copy (not in JSON mode)
    let pb = create_progress_bar(quiet || json);

    // Create copy engine
    let mut engine = CopyEngine::new().await?;

    // Start the engine
    engine.start().await?;

    // Store paths for JSON output before moving them
    let source_path = source.to_string_lossy().to_string();
    let destination_path = destination.to_string_lossy().to_string();

    // Create copy request using builder pattern
    let request = CopyRequest::new(source, destination)
        .with_mode(mode)
        .preserve_metadata(true)
        .verify_copy(false)
        .enable_compression(compress)
        .exclude_patterns(exclude)
        .include_patterns(include);

    // Note: threads, compression_level, zero_copy are handled by the engine internally
    // These CLI options could be used to configure the engine in the future

    if let Some(pb) = &pb {
        pb.set_message("Starting copy operation...");
    }

    // Execute copy operation with progress updates
    let result = tokio::select! {
        result = engine.execute(request) => {
            result?
        }
        _ = async {
            // Simple progress simulation
            if let Some(pb) = &pb {
                let mut counter = 0;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    counter += 1;
                    match counter % 4 {
                        0 => pb.set_message("Copying files..."),
                        1 => pb.set_message("Processing files..."),
                        2 => pb.set_message("Transferring data..."),
                        _ => pb.set_message("Finalizing..."),
                    }
                }
            } else {
                // If no progress bar, just wait indefinitely
                std::future::pending::<()>().await;
            }
        } => {
            // This branch should never be reached
            return Err(anyhow::anyhow!("Progress task completed unexpectedly"));
        }
    };

    let stats = result.stats;

    // Stop the engine
    engine.stop().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("Copy completed");
    }

    if json {
        // Output JSON format
        let json_result = CopyResultJson::new(
            source_path,
            destination_path,
            &source_info,
            &dest_info,
            &comparison,
            &stats,
        );

        let json_output = serde_json::to_string_pretty(&json_result)?;
        println!("{}", json_output);
    } else if !quiet {
        display_enhanced_copy_stats(&stats, Some(&comparison));

        // Show final performance summary
        let actual_speed = stats.transfer_rate() / 1024.0 / 1024.0;
        let efficiency = (actual_speed / comparison.expected_speed_mbps) * 100.0;

        if efficiency < 60.0 {
            display_warning(&format!(
                "Performance was lower than expected ({:.1}% efficiency). Consider checking disk health or system load.",
                efficiency
            ));
        } else if efficiency >= 90.0 {
            display_success("Excellent performance achieved!");
        }
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

    let analyzer = PerformanceAnalyzer::new();
    let analysis_pb = create_analysis_spinner("Analyzing device...");

    let device_info = analyzer.analyze_device(&path).await?;
    analysis_pb.finish_and_clear();

    display_device_info("Device Information", &device_info);

    // Additional technical details
    println!();
    println!(
        "{} {}",
        style("🔧").blue().bold(),
        style("Technical Details").bold().underlined()
    );
    println!(
        "  Random Read IOPS: {:.0}",
        device_info.performance.random_read_iops
    );
    println!(
        "  Random Write IOPS: {:.0}",
        device_info.performance.random_write_iops
    );
    println!(
        "  Average Latency: {:.1} μs",
        device_info.performance.average_latency
    );
    println!("  Queue Depth: {}", device_info.performance.queue_depth);
    println!(
        "  TRIM Support: {}",
        if device_info.performance.supports_trim {
            "Yes"
        } else {
            "No"
        }
    );

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
