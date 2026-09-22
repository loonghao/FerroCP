//! Build script for the ferrocp-python crate.

fn main() {
    // Linking against Python is owned entirely by pyo3's own build script: it
    // emits `rustc-link-search` for the interpreter's `libs` directory and
    // `rustc-link-lib=pythonXY:python3` for the abi3 import library.
    //
    // This script previously emitted its own unconditional
    // `cargo:rustc-link-lib=dylib=python3` on Windows. That duplicated pyo3's
    // directive without the `pythonXY:` linker modifier, so the linker searched
    // for a `python3.lib` it could not resolve and failed with LNK1181.
    // Nothing here needs to run, so the script is intentionally empty.
}
