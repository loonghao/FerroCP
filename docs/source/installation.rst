Installation
============

Stable release
-------------

To install FerroCP, run this command in your terminal:

.. code-block:: console

    $ pip install ferrocp

Or with uv:

.. code-block:: console

    $ uv add ferrocp

This is the preferred method to install FerroCP, as it will always install the most recent stable release.

From sources
-----------

The sources for FerroCP can be downloaded from the `Github repo`_.

You can either clone the public repository:

.. code-block:: console

    $ git clone https://github.com/loonghao/ferrocp.git

Or download the `tarball`_:

.. code-block:: console

    $ curl -OJL https://github.com/loonghao/ferrocp/tarball/main

Once you have a copy of the source, you can install it with:

.. code-block:: console

    $ uv sync
    $ uv run maturin develop --release


.. _Github repo: https://github.com/loonghao/ferrocp
.. _tarball: https://github.com/loonghao/ferrocp/tarball/main
