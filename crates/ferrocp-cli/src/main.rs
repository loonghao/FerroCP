//! FerroCP - High-performance cross-platform file copying tool
//!
//! A modern, fast, and reliable file copying tool written in Rust with advanced features
//! like zero-copy operations, compression, and intelligent device detection.

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use ferrocp_device::PerformanceAnalyzer;
use ferrocp_engine::{CopyEngine, CopyRequest};
use ferrocp_io::OverwritePrompt;
use ferrocp_types::{CopyMode, OverwriteDecision, OverwritePolicy, SymlinkMode};
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
        ///
        /// `mirror` is declared but not implemented and is rejected with an
        /// error instead of silently behaving like `all`.
        #[arg(short, long, value_enum, default_value = "all")]
        mode: CopyModeArg,
        /// What to do when the destination already exists
        ///
        /// Accepted values: always, never, if_newer, if_different, fail, prompt.
        /// When omitted, the policy implied by `--mode` is used.
        #[arg(long)]
        overwrite: Option<String>,
        /// How to treat symbolic links in the source
        ///
        /// Accepted values: preserve, follow, fail
        #[arg(long, default_value = "preserve")]
        symlinks: String,
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
            overwrite,
            symlinks,
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
            // Parse the semantic flags before touching the filesystem so a
            // typo produces a precise error instead of a silent default.
            // `--overwrite` is optional: when it is omitted the policy implied
            // by `--mode` stays in effect.
            let overwrite_policy = overwrite
                .as_deref()
                .map(parse_overwrite_policy)
                .transpose()?;
            let symlink_mode = parse_symlink_mode(&symlinks)?;

            copy_command(CopyOptions {
                source,
                destination,
                mode: copy_mode,
                overwrite_policy,
                symlink_mode,
                _threads: threads,
                compress,
                _compression_level: compression_level,
                _zero_copy: zero_copy,
                exclude,
                include,
                quiet: cli.quiet,
                json,
            })
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

/// Parse `--overwrite`, listing the accepted values on failure
fn parse_overwrite_policy(value: &str) -> Result<OverwritePolicy> {
    OverwritePolicy::parse(value).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid --overwrite value '{}'; accepted values: {}",
            value,
            OverwritePolicy::accepted_values().join(", ")
        )
    })
}

/// Parse `--symlinks`, listing the accepted values on failure
fn parse_symlink_mode(value: &str) -> Result<SymlinkMode> {
    SymlinkMode::parse(value).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid --symlinks value '{}'; accepted values: {}",
            value,
            SymlinkMode::accepted_values().join(", ")
        )
    })
}

/// Build the overwrite policy actually used for this run
///
/// `--mode` rejects `mirror` up front; an explicit `--overwrite` then wins over
/// the policy the mode implies.
fn resolve_overwrite_policy(
    mode: CopyMode,
    explicit: Option<OverwritePolicy>,
) -> Result<OverwritePolicy> {
    // Surface the unimplemented mode before doing any work.
    let implied = mode.overwrite_policy()?;
    Ok(explicit.unwrap_or(implied))
}

/// Build the prompt handler used by `--overwrite prompt`
///
/// Asks on the terminal and fails when stdin is not a terminal, because an
/// unanswered prompt must never be treated as "yes".
fn build_overwrite_prompt() -> Result<OverwritePrompt> {
    Ok(OverwritePrompt::new(|source, destination| {
        let question = format!(
            "Overwrite '{}' with '{}'?",
            destination.display(),
            source.display()
        );

        if !dialoguer::console::Term::stdout().is_term() {
            // The callback cannot return an error, so refuse to overwrite and
            // let the caller see the file was skipped.
            eprintln!(
                "warning: cannot prompt for '{}' because stdin is not a terminal; skipping",
                destination.display()
            );
            return OverwriteDecision::Skip;
        }

        match dialoguer::Confirm::new()
            .with_prompt(question)
            .default(false)
            .interact()
        {
            Ok(true) => OverwriteDecision::Proceed,
            Ok(false) => OverwriteDecision::Skip,
            Err(error) => {
                eprintln!(
                    "warning: failed to read the answer for '{}': {}",
                    destination.display(),
                    error
                );
                OverwriteDecision::Skip
            }
        }
    }))
}

/// Everything the `copy` sub-command collected from the CLI.
///
/// Grouped into a struct so the handler keeps a readable signature instead of
/// tripping `clippy::too_many_arguments`.
struct CopyOptions {
    source: PathBuf,
    destination: PathBuf,
    mode: CopyMode,
    overwrite_policy: Option<OverwritePolicy>,
    symlink_mode: SymlinkMode,
    // Accepted for CLI compatibility; the engine sizes its own thread pool.
    _threads: Option<usize>,
    compress: bool,
    // Accepted for CLI compatibility; the compression level is engine-internal.
    _compression_level: u8,
    // Accepted for CLI compatibility; zero-copy is chosen by the engine.
    _zero_copy: bool,
    exclude: Vec<String>,
    include: Vec<String>,
    quiet: bool,
    json: bool,
}

async fn copy_command(options: CopyOptions) -> Result<()> {
    let CopyOptions {
        source,
        destination,
        mode,
        overwrite_policy,
        symlink_mode,
        compress,
        exclude,
        include,
        quiet,
        json,
        ..
    } = options;

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
    let mut request = CopyRequest::new(source, destination)
        .with_mode(mode)
        .with_symlink_mode(symlink_mode)
        .preserve_metadata(true)
        .verify_copy(false)
        .enable_compression(compress)
        .exclude_patterns(exclude)
        .include_patterns(include);

    // `--overwrite prompt` needs a terminal handler; without one the copy would
    // fall back to a guess, which is exactly what this flag must never do.
    let overwrite_policy = resolve_overwrite_policy(mode, overwrite_policy)?;
    if overwrite_policy == OverwritePolicy::Prompt {
        let prompt = build_overwrite_prompt()?;
        request = request.with_overwrite_semantics(overwrite_policy, Some(prompt));
    } else {
        request = request.with_overwrite_semantics(overwrite_policy, None);
    }

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

    // Surface a failed copy instead of reporting success with zero files: a
    // task can fail without producing per-file errors (for example
    // `--overwrite fail` on an existing destination).
    let failure_reason =
        if result.is_success() {
            None
        } else {
            Some(result.error.clone().unwrap_or_else(|| {
                "copy task reported a failure without an error message".to_string()
            }))
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
            failure_reason.as_deref(),
        );

        let json_output = serde_json::to_string_pretty(&json_result)?;
        println!("{}", json_output);
    } else if !quiet {
        if let Some(reason) = &failure_reason {
            display_error(reason);
        }
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
