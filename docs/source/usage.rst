Usage
=====

Python API
----------

Every copy helper is asynchronous and must be awaited, including the
``shutil``-style aliases ``copy``, ``copy2`` and ``copytree``::

    import asyncio
    import ferrocp

    async def main():
        # shutil-style aliases
        await ferrocp.copy("source.txt", "destination.txt")
        await ferrocp.copy2("source.txt", "destination.txt")
        await ferrocp.copytree("source_dir", "destination_dir")

        # Explicit helper with options
        options = ferrocp.CopyOptions(
            verify=True,
            preserve_timestamps=True,
            preserve_permissions=True,
            enable_compression=True,
        )
        result = await ferrocp.copy_file("large.bin", "backup.bin", options=options)
        print(result.success, result.bytes_copied, result.duration_seconds)

    asyncio.run(main())

``CopyOptions`` keyword arguments (defaults in parentheses): ``mode``
(``"auto"``), ``overwrite`` (``"prompt"``), ``preserve_timestamps`` (``True``),
``preserve_permissions`` (``True``), ``follow_symlinks`` (``False``),
``enable_compression`` (``False``), ``compression_level`` (``6``),
``buffer_size`` (``65536``), ``num_threads`` (``0``) and
``verify`` (``False``).

Only ``verify``, ``preserve_timestamps``, ``preserve_permissions`` and
``enable_compression`` affect the copy; the remaining fields are accepted but
ignored by the current implementation.

A ``CopyResult`` exposes ``bytes_copied``, ``files_copied``,
``duration_seconds``, ``transfer_rate``, ``success`` and ``error_message``.

Command Line Interface
----------------------

Two different ``ferrocp`` commands exist. The Rust CLI comes from
``crates/ferrocp-cli``; the Python CLI is the ``ferrocp`` console script
defined by ``python/ferrocp/cli.py``.

Rust CLI
~~~~~~~~

Global options must be placed **before** the subcommand::

    $ ferrocp --help
    Usage: ferrocp [OPTIONS] <COMMAND>

    Commands:
      copy    Copy files and directories
      sync    Synchronize directories
      verify  Verify file integrity
      device  Show device information
      config  Show configuration
      help    Print this message or the help of the given subcommand(s)

    Options:
      -d, --debug
      -q, --quiet
      -v, --verbose
      -c, --config <CONFIG>
      -h, --help
      -V, --version

Copy a file or a directory::

    $ ferrocp copy source.txt destination.txt
    $ ferrocp copy --mirror source_dir/ destination_dir/
    $ ferrocp copy source_dir/ destination_dir/ --json

``ferrocp copy`` options:

.. code-block:: text

    -m, --mode <MODE>              all (default), newer, different, mirror
    -t, --threads <THREADS>        accepted, but not wired to the engine yet
        --compress                 enable compression
        --compression-level <N>    0-22, default 6; accepted, but not wired to the engine yet
        --zero-copy                enable zero-copy operations
        --mirror                   mirror mode, overrides --mode
        --exclude <PATTERN>        repeatable
        --include <PATTERN>        repeatable
        --json                     emit the JSON result document

.. note::

   ``sync``, ``verify`` and ``config`` are parsed but only print a placeholder
   message; the underlying logic is still unimplemented in
   ``crates/ferrocp-cli/src/main.rs``.

Python CLI
~~~~~~~~~~

::

    $ ferrocp --version
    $ ferrocp --verbose copy SOURCE DESTINATION --threads 4 --buffer-size 8388608
    $ ferrocp copy_with_server SOURCE DESTINATION --server HOST --port 8080
    $ ferrocp benchmark
