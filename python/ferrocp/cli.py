"""Command-line interface for ferrocp."""

import sys
import time
from pathlib import Path
from typing import Optional

import click

from . import EACopy, __version__


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

    # Create EACopy instance with configuration
    eacopy = EACopy(
        thread_count=threads,
        buffer_size=buffer_size,
        compression_level=compression,
        preserve_metadata=preserve_metadata,
        follow_symlinks=follow_symlinks,
    )

    # Set up progress callback if requested
    if progress:
        def progress_callback(current: int, total: int, filename: str) -> None:
            if total > 0:
                percent = (current / total) * 100
                click.echo(f"\rProgress: {percent:.1f}% - {filename}", nl=False)
            else:
                click.echo(f"\rCopying: {filename}", nl=False)

        # Note: Progress callback is set during EACopy construction
        eacopy = EACopy(
            thread_count=threads,
            buffer_size=buffer_size,
            compression_level=compression,
            preserve_metadata=preserve_metadata,
            follow_symlinks=follow_symlinks,
            progress_callback=progress_callback,
        )

    try:
        start_time = time.time()

        if source.is_file():
            stats = eacopy.copy_file(str(source), str(destination))
        else:
            stats = eacopy.copy_directory(str(source), str(destination))

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

        if stats.zerocopy_used > 0:
            zerocopy_percent = (stats.zerocopy_bytes / stats.bytes_copied) * 100
            click.echo(f"  Zero-copy: {zerocopy_percent:.1f}% of data")

        if stats.errors > 0:
            click.echo(f"  Errors: {stats.errors}", err=True)
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

    eacopy = EACopy()

    try:
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
        eacopy = EACopy()

        for filename, size in test_files:
            source = test_dir / filename
            dest = test_dir / f"copy_{filename}"

            start_time = time.time()
            eacopy.copy_file(str(source), str(dest))
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
