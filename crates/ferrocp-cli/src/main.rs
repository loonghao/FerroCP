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
use ferrocp_types::{
    CompressionLevel, CopyMode, OverwriteDecision, OverwritePolicy, SymlinkMode, ThreadCount,
};
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
    ///
    /// Read by `copy` (it overrides the configuration file that would
    /// otherwise be discovered) and by `config` (it is the file printed).
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
        /// Number of worker threads used for concurrent copies
        ///
        /// Accepted values are 1-256. When omitted the engine sizes its own
        /// pool from the available parallelism.
        #[arg(short, long)]
        threads: Option<usize>,
        /// Enable compression
        ///
        /// Forwarded to the I/O layer, which does not compress yet: the flag
        /// changes the request but not the bytes that are written.
        #[arg(long)]
        compress: bool,
        /// Compression level (0-22)
        ///
        /// Not implemented: the I/O layer has no compressor, so passing this
        /// option exits with an error instead of being ignored.
        #[arg(long)]
        compression_level: Option<u8>,
        /// Enable zero-copy operations
        ///
        /// Not implemented: the I/O layer always uses a buffered copy, so
        /// passing this flag exits with an error instead of being ignored.
        #[arg(long)]
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
    /// Synchronize directories (not implemented yet)
    ///
    /// Parsed, but rejected at run time: the sync logic is still missing, and
    /// reporting success without doing anything would be a lie.
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
    /// Verify file integrity (not implemented yet)
    ///
    /// Parsed, but rejected at run time: no comparison is performed yet.
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
    ///
    /// Prints the effective configuration as JSON: the built-in defaults with
    /// `--default`, otherwise the defaults overlaid with the first configuration
    /// file found and the `FERROCP_` environment variables.
    Config {
        /// Show the built-in defaults instead of the effective configuration
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
            // Reject the tuning options the copy path cannot honour before
            // touching the filesystem, so a run never claims an effect it does
            // not have.
            reject_unimplemented_tuning(compression_level, zero_copy)?;

            copy_command(CopyOptions {
                source,
                destination,
                mode: copy_mode,
                overwrite_policy,
                symlink_mode,
                engine_config: engine_config(cli.config.as_deref(), threads)?,
                compress,
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
            config_command(default, cli.config.as_deref()).await?;
        }
    }

    Ok(())
}

/// Build the engine configuration the copy path runs with
///
/// The starting point is the configuration the rest of FerroCP reads: the
/// built-in defaults, then the configuration file (`--config <PATH>` when
/// given, otherwise the first one found) and the `FERROCP__` environment
/// variables. Building it from `Config::default()` would silently drop a user's
/// `ferrocp.toml` and environment overrides for every copy.
///
/// `--threads` is then honoured on top: it bounds how many copy tasks the
/// engine runs concurrently. `ThreadCount` rejects out-of-range values, so the
/// range in the error message is the one the engine actually accepts.
fn engine_config(
    config_path: Option<&std::path::Path>,
    threads: Option<usize>,
) -> Result<ferrocp_config::Config> {
    let mut config = match config_path {
        Some(path) => ferrocp_config::ConfigLoader::load_from_file(path).map_err(|error| {
            anyhow::anyhow!(
                "cannot load configuration from '{}': {error}",
                path.display()
            )
        })?,
        None => ferrocp_config::ConfigLoader::load_default()
            .map_err(|error| anyhow::anyhow!("cannot load the default configuration: {error}"))?,
    };

    if let Some(threads) = threads {
        let thread_count = ThreadCount::new(threads)
            .map_err(|error| anyhow::anyhow!("invalid --threads value {threads}: {error}"))?;
        config.performance.thread_count = thread_count;
    }

    Ok(config)
}

/// Refuse the `copy` options that cannot change how the copy runs
///
/// FerroCP has no compressor and no zero-copy path in its I/O layer, so both
/// options would be accepted and then ignored. Failing here keeps the promise
/// the CLI makes: an option either changes the copy or is refused.
fn reject_unimplemented_tuning(compression_level: Option<u8>, zero_copy: bool) -> Result<()> {
    if let Some(level) = compression_level {
        // Range first: a level above 22 is a typo, not an unsupported feature.
        CompressionLevel::new(level).map_err(|error| {
            anyhow::anyhow!("invalid --compression-level value {level}: {error}")
        })?;
        anyhow::bail!(
            "--compression-level is not implemented: the I/O layer has no compressor, so the level \
             is never applied. Drop the option to copy without compression."
        );
    }

    if zero_copy {
        anyhow::bail!(
            "--zero-copy is not implemented: every copy goes through the buffered engine, so the \
             flag has no effect. Drop the flag to copy with the buffered engine."
        );
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
    /// Engine configuration derived from the tuning options (`--threads`)
    engine_config: ferrocp_config::Config,
    compress: bool,
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
        engine_config,
        compress,
        exclude,
        include,
        quiet,
        json,
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

    // Create copy engine with the configuration the tuning options asked for
    let mut engine = CopyEngine::with_config(engine_config).await?;

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

    // A failed copy must not exit 0.
    //
    // Everything above has already been printed, including the JSON document
    // with its `failure_reason`, so returning an error here only changes the
    // exit status. Leaving it as `Ok(())` made a failed copy indistinguishable
    // from a successful one for scripts: the stats print either way and the
    // status code was the only signal a caller could act on.
    //
    // ERROR_MODEL.md principle 1 - never swallow an error - applies to the CLI
    // as much as to the library.
    if let Some(reason) = failure_reason {
        return Err(anyhow::anyhow!("{reason}"));
    }

    info!("Copy operation completed successfully");
    Ok(())
}

/// Reject a sub-command whose logic does not exist yet
///
/// Printing "completed" for an operation that did nothing is worse than an
/// error: a script cannot tell the difference, and the next step in a pipeline
/// would read data that was never produced.
fn unimplemented_command(command: &str, what: &str) -> Result<()> {
    anyhow::bail!(
        "`ferrocp {command}` is not implemented yet: {what}. The sub-command is parsed so scripts \
         can be written against it, but it performs no work and exits with this error instead of \
         reporting success."
    );
}

async fn sync_command(
    _source: PathBuf,
    _destination: PathBuf,
    _dry_run: bool,
    _delete: bool,
) -> Result<()> {
    unimplemented_command(
        "sync",
        "no synchronization is performed, so the destination would be left untouched",
    )
}

async fn verify_command(_path: PathBuf, _source: Option<PathBuf>) -> Result<()> {
    unimplemented_command(
        "verify",
        "no integrity check is performed, so nothing would be compared",
    )
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

/// Print the configuration as JSON
///
/// `--default` prints the built-in defaults. Otherwise the effective
/// configuration is loaded the same way the copy path loads it: defaults, then
/// the first configuration file found (`--config <PATH>` when given), then the
/// `FERROCP_` environment variables.
async fn config_command(default: bool, config_path: Option<&std::path::Path>) -> Result<()> {
    if default {
        let config = ferrocp_config::Config::default();
        println!("{} Default configuration:", style("⚙").blue().bold());
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }

    let config = match config_path {
        Some(path) => ferrocp_config::ConfigLoader::load_from_file(path).map_err(|error| {
            anyhow::anyhow!(
                "cannot load configuration from '{}': {error}",
                path.display()
            )
        })?,
        None => ferrocp_config::ConfigLoader::load_default()
            .map_err(|error| anyhow::anyhow!("cannot load the default configuration: {error}"))?,
    };

    let origin = match config_path {
        Some(path) => format!("'{}'", path.display()),
        None => match ferrocp_config::ConfigLoader::config_exists() {
            Some(path) => format!("'{}'", path.display()),
            None => "no configuration file found".to_string(),
        },
    };

    println!(
        "{} Effective configuration ({} plus FERROCP_ environment variables):",
        style("⚙").blue().bold(),
        origin
    );
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `--threads` must reach the engine configuration: it bounds how many
    /// copies run concurrently.
    #[test]
    fn engine_config_applies_the_thread_count() {
        let config = engine_config(None, Some(4)).expect("4 is a valid thread count");
        assert_eq!(config.performance.thread_count.get(), 4);
    }

    /// Omitting `--threads` keeps the engine's own sizing.
    #[test]
    fn engine_config_without_threads_keeps_the_default() {
        let config = engine_config(None, None).expect("the default configuration is valid");
        assert_eq!(config.performance.thread_count, ThreadCount::default());
    }

    /// Out-of-range values are refused instead of being clamped, so the number
    /// the user sees is the number the engine uses.
    #[test]
    fn engine_config_rejects_an_out_of_range_thread_count() {
        let error = engine_config(None, Some(0)).expect_err("0 threads is below the minimum");
        assert!(
            error.to_string().contains("--threads"),
            "the error must name the option: {error}"
        );

        let error =
            engine_config(None, Some(usize::MAX)).expect_err("the count is above the maximum");
        assert!(error.to_string().contains("--threads"));
    }

    /// The copy path must read the configuration file a user wrote, not just
    /// the built-in defaults. Regression guard: an earlier revision built the
    /// engine from `Config::default()`, which silently dropped `ferrocp.toml`
    /// and every `FERROCP__` override.
    #[test]
    fn engine_config_reads_the_configuration_file() {
        let temp_dir = tempfile::tempdir().expect("a temporary directory");
        let path = temp_dir.path().join("ferrocp.toml");
        std::fs::write(
            &path,
            "[performance]\nbuffer_size = 131072\nthread_count = 3\n",
        )
        .expect("the configuration file is writable");

        let config = engine_config(Some(&path), None).expect("the configuration file is valid");
        assert_eq!(
            config.performance.buffer_size.get(),
            131072,
            "the buffer size from the file must reach the engine"
        );
        assert_eq!(config.performance.thread_count.get(), 3);

        // `--threads` still wins over the file: it is the more specific request.
        let config = engine_config(Some(&path), Some(8)).expect("8 is a valid thread count");
        assert_eq!(config.performance.thread_count.get(), 8);
        assert_eq!(config.performance.buffer_size.get(), 131072);
    }

    /// A `--config` path that does not exist is an error, not a silent fallback
    /// to the defaults.
    #[test]
    fn engine_config_rejects_a_missing_configuration_file() {
        let error = engine_config(Some(std::path::Path::new("does/not/exist.toml")), None)
            .expect_err("a missing file must not be ignored");
        assert!(
            error.to_string().contains("cannot load configuration from"),
            "the error must name the failing path: {error}"
        );
    }

    /// Neither option can change the copy, so both must fail rather than be
    /// ignored.
    #[test]
    fn tuning_options_that_cannot_be_honoured_are_rejected() {
        let error = reject_unimplemented_tuning(Some(6), false).expect_err("no compressor exists");
        assert!(
            error.to_string().contains("--compression-level"),
            "the error must name the option: {error}"
        );

        let error = reject_unimplemented_tuning(None, true).expect_err("no zero-copy path exists");
        assert!(
            error.to_string().contains("--zero-copy"),
            "the error must name the option: {error}"
        );

        assert!(reject_unimplemented_tuning(None, false).is_ok());
    }

    /// A level above the supported range is a typo, not a missing feature, and
    /// must say so.
    #[test]
    fn compression_level_reports_an_out_of_range_value_as_invalid() {
        let error = reject_unimplemented_tuning(Some(23), false).expect_err("23 is out of range");
        assert!(
            error.to_string().contains("invalid --compression-level"),
            "the error must report the invalid value: {error}"
        );
    }

    /// A sub-command with no logic must not report success.
    #[test]
    fn unimplemented_sub_commands_fail_instead_of_lying() {
        let error =
            unimplemented_command("sync", "no synchronization is performed").expect_err("no sync");
        assert!(error.to_string().contains("not implemented"));

        let error =
            unimplemented_command("verify", "no check is performed").expect_err("no verify");
        assert!(error.to_string().contains("not implemented"));
    }
}
