"""Command-line interface for ferrocp."""

import asyncio
import sys
import time
from pathlib import Path
from typing import Optional

import click

from . import CopyEngine, CopyOptions, __version__


@click.group()
@click.version_option(version=__version__, prog_name="ferrocp")
@click.option("--verbose", "-v", is_flag=True, help="Enable verbose output")
@click.pass_context
def cli(ctx: click.Context, verbose: bool) -> None:
    """High-performance file copying with Rust implementation."""
    ctx.ensure_object(dict)
    ctx.obj["verbose"] = verbose


@cli.command()
@click.argument("source", type=click.Path(exists=True, path_type=Path))
@click.argument("destination", type=click.Path(path_type=Path))
@click.option("--threads", "-t", type=int, default=4, help="Number of threads to use")
@click.option("--buffer-size", "-b", type=int, default=8*1024*1024, help="Buffer size in bytes")
@click.option("--compression", "-c", type=int, default=0, help="Compression level (0-9)")
@click.option("--preserve-metadata/--no-preserve-metadata", default=True, help="Preserve file metadata")
@click.option("--follow-symlinks/--no-follow-symlinks", default=False, help="Follow symbolic links")
@click.option("--zerocopy/--no-zerocopy", default=True, help="Enable zero-copy operations")
@click.option("--progress/--no-progress", default=True, help="Show progress")
@click.pass_context
def copy(
    ctx: click.Context,
    source: Path,
    destination: Path,
    threads: int,
    buffer_size: int,
    compression: int,
    preserve_metadata: bool,
    follow_symlinks: bool,
    zerocopy: bool,
    progress: bool,
) -> None:
    """Copy a file or directory."""
    verbose = ctx.obj.get("verbose", False)

    if verbose:
        click.echo(f"Copying {source} to {destination}")
        click.echo(f"Threads: {threads}, Buffer: {buffer_size}, Compression: {compression}")

    # Create CopyEngine instance with configuration
    engine = CopyEngine()
    options = CopyOptions()
    options.num_threads = threads
    options.buffer_size = buffer_size
    options.compression_level = compression
    options.enable_compression = compression > 0
    options.preserve_timestamps = preserve_metadata
    options.follow_symlinks = follow_symlinks

    # Set up progress callback if requested
    if progress:
        def progress_callback(current: int, total: int, filename: str) -> None:
            if total > 0:
                percent = (current / total) * 100
                click.echo(f"\rProgress: {percent:.1f}% - {filename}", nl=False)
            else:
                click.echo(f"\rCopying: {filename}", nl=False)

        # Note: Progress callback would be set on options
        # options.progress_callback = progress_callback  # TODO: Implement progress callback

    try:
        start_time = time.time()

        # Run the async copy operation
        async def run_copy():
            if source.is_file():
                return await engine.copy_file(str(source), str(destination), options)
            else:
                return await engine.copy_directory(str(source), str(destination), options)

        stats = asyncio.run(run_copy())
        end_time = time.time()

        if progress:
            click.echo()  # New line after progress

        # Display results
        duration = end_time - start_time
        speed_mbps = (stats.bytes_copied / (1024 * 1024)) / duration if duration > 0 else 0

        click.echo("✓ Copy completed successfully!")
        click.echo(f"  Files copied: {stats.files_copied}")
        click.echo(f"  Bytes copied: {stats.bytes_copied:,}")
        click.echo(f"  Duration: {duration:.2f}s")
        click.echo(f"  Speed: {speed_mbps:.2f} MB/s")

        # Check if operation was successful
        if not stats.success:
            error_msg = stats.error_message or "Unknown error"
            click.echo(f"  Error: {error_msg}", err=True)
            sys.exit(1)

    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.argument("source", type=click.Path(exists=True, path_type=Path))
@click.argument("destination", type=click.Path(path_type=Path))
@click.option("--server", "-s", help="Server address for network acceleration")
@click.option("--port", "-p", type=int, default=8080, help="Server port")
@click.pass_context
def copy_with_server(
    ctx: click.Context,
    source: Path,
    destination: Path,
    server: Optional[str],
    port: int,
) -> None:
    """Copy files using network server acceleration."""
    verbose = ctx.obj.get("verbose", False)

    if not server:
        click.echo("Error: Server address is required for network copy", err=True)
        sys.exit(1)

    if verbose:
        click.echo(f"Copying {source} to {destination} via server {server}:{port}")

    engine = CopyEngine()
    options = CopyOptions()

    try:
        # Use EACopy for server-based copying
        from . import EACopy
        eacopy = EACopy(thread_count=4, buffer_size=64*1024)
        stats = eacopy.copy_with_server(str(source), str(destination), server, port)
        click.echo(f"✓ Network copy completed! Copied {stats.bytes_copied:,} bytes")
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)


@cli.command()
def benchmark() -> None:
    """Run performance benchmarks."""
    click.echo("Running ferrocp benchmarks...")

    # Create test data
    test_dir = Path("benchmark_test")
    test_dir.mkdir(exist_ok=True)

    try:
        # Create test files of different sizes
        test_files = [
            ("small.txt", 1024),          # 1KB
            ("medium.txt", 1024 * 1024),  # 1MB
            ("large.txt", 10 * 1024 * 1024),  # 10MB
        ]

        for filename, size in test_files:
            test_file = test_dir / filename
            with open(test_file, "wb") as f:
                f.write(b"x" * size)

        # Run benchmarks
        engine = CopyEngine()
        options = CopyOptions()

        for filename, size in test_files:
            source = test_dir / filename
            dest = test_dir / f"copy_{filename}"

            start_time = time.time()

            # Run async copy operation
            async def run_benchmark_copy():
                return await engine.copy_file(str(source), str(dest), options)

            asyncio.run(run_benchmark_copy())
            duration = time.time() - start_time

            speed_mbps = (size / (1024 * 1024)) / duration if duration > 0 else 0

            click.echo(f"{filename}: {speed_mbps:.2f} MB/s")

            # Clean up
            dest.unlink(missing_ok=True)

    finally:
        # Clean up test directory
        for file in test_dir.glob("*"):
            file.unlink()
        test_dir.rmdir()


def main() -> None:
    """Run the main CLI entry point."""
    cli()


if __name__ == "__main__":
    main()
