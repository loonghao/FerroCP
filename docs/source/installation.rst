Installation
============

.. warning::

   No FerroCP distribution is published yet: the ``ferrocp`` package is not on
   PyPI, and no prebuilt CLI archive is attached to a GitHub release. Install
   from sources as described below.

From sources
-----------

The sources for FerroCP can be downloaded from the `Github repo`_.

You can either clone the public repository:

.. code-block:: console

    $ git clone https://github.com/loonghao/ferrocp.git

Or download the `tarball`_:

.. code-block:: console

    $ curl -OJL https://github.com/loonghao/ferrocp/tarball/main

Once you have a copy of the source, you can install the Python package with:

.. code-block:: console

    $ uv sync
    $ uv run maturin develop --release

To install the standalone Rust CLI instead (no Python dependency):

.. code-block:: console

    $ cargo build --release --bin ferrocp

Requirements: Python 3.9 or newer (the extension is built with ``abi3-py39``)
and a Rust toolchain from https://rustup.rs.


.. _Github repo: https://github.com/loonghao/ferrocp
.. _tarball: https://github.com/loonghao/ferrocp/tarball/main
