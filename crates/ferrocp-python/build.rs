//! Build script for ferrocp-python crate
//!
//! This build script configures the Python extension module build process.

fn main() {
    // On Windows, Python extension modules should have .pyd extension
    // PyO3 handles this automatically, but we ensure proper configuration

    #[cfg(target_os = "windows")]
    {
        // Tell cargo to link against Python DLL
        // This is handled by PyO3, but we can add additional configuration if needed
        println!("cargo:rustc-link-lib=dylib=python3");
    }

    // Disable tests for cdylib crates as they cause DLL dependency issues
    println!("cargo:rustc-cfg=disable_tests");
}
