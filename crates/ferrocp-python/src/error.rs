//! Error handling for Python bindings

use ferrocp_types::Error;
use pyo3::prelude::*;
use pyo3::{create_exception, exceptions::PyException};

// Define custom Python exceptions
// Note: create_exception! macro doesn't support documentation
#[allow(missing_docs)]
mod exceptions {
    use super::*;

    create_exception!(ferrocp_python, PyFerrocpError, PyException);
    create_exception!(ferrocp_python, PyIoError, PyFerrocpError);
    create_exception!(ferrocp_python, PyConfigError, PyFerrocpError);
    create_exception!(ferrocp_python, PyNetworkError, PyFerrocpError);
    create_exception!(ferrocp_python, PySyncError, PyFerrocpError);

    // These derive from the matching builtins so `except FileNotFoundError`
    // and `except PermissionError` work for callers. They intentionally do not
    // derive from PyFerrocpError: Python has single inheritance here, and
    // matching the stdlib contract is what users catch.
    create_exception!(
        ferrocp_python,
        PyFileNotFoundError,
        pyo3::exceptions::PyFileNotFoundError
    );
    create_exception!(
        ferrocp_python,
        PyPermissionError,
        pyo3::exceptions::PyPermissionError
    );
}

pub use exceptions::*;

/// Render an error for Python, including its context chain
///
/// The chain is part of the message because Python callers only see the
/// exception text: "copy_file: I/O error: ..." is actionable, "I/O error: ..."
/// alone is not.
fn error_message(error: &Error) -> String {
    error.chain()
}

/// Error wrapper for Python bindings
#[derive(Debug)]
pub struct PyErrorWrapper(pub Error);

impl From<Error> for PyErrorWrapper {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

/// Convert Rust Error to Python exception
impl From<PyErrorWrapper> for PyErr {
    fn from(wrapper: PyErrorWrapper) -> Self {
        match wrapper.0 {
            // Errors that carry an os-level kind map onto the matching builtin
            // exception, so `except OSError` and friends work as users expect.
            Error::Io { message, kind } => match kind {
                Some(std::io::ErrorKind::NotFound) => PyFileNotFoundError::new_err(message),
                Some(std::io::ErrorKind::PermissionDenied) => PyPermissionError::new_err(message),
                _ => PyIoError::new_err(error_message(&Error::Io { message, kind })),
            },
            Error::FileNotFound { path } => {
                PyFileNotFoundError::new_err(format!("File not found: {}", path.display()))
            }
            Error::PermissionDenied { path } => {
                PyPermissionError::new_err(format!("Permission denied: {}", path.display()))
            }
            Error::WithContext { error, context } => {
                let mut err = PyErr::from(PyErrorWrapper(*error));
                if let Some(path) = &context.path {
                    err = PyErr::new::<PyFerrocpError, _>(format!(
                        "{} (while {} '{}')",
                        err,
                        context.operation,
                        path.display()
                    ));
                }
                err
            }
            Error::Config { message } => PyConfigError::new_err(message),
            Error::Network { message } => PyNetworkError::new_err(message),
            Error::Sync { message } => PySyncError::new_err(message),
            Error::Compression { message } => {
                PyFerrocpError::new_err(format!("Compression error: {}", message))
            }
            Error::DeviceDetection { message } => {
                PyFerrocpError::new_err(format!("Device detection error: {}", message))
            }
            Error::ZeroCopy { message } => {
                PyFerrocpError::new_err(format!("Zero-copy error: {}", message))
            }
            Error::Cancelled => PyFerrocpError::new_err("Operation cancelled"),
            Error::Timeout { seconds } => {
                PyNetworkError::new_err(format!("Operation timed out after {} seconds", seconds))
            }
            Error::Other { message } => PyFerrocpError::new_err(message),
        }
    }
}

/// Helper trait for converting Results
pub trait IntoPyResult<T> {
    /// Convert a Rust Result to a Python Result
    fn into_py_result(self) -> PyResult<T>;
}

impl<T> IntoPyResult<T> for Result<T, Error> {
    fn into_py_result(self) -> PyResult<T> {
        self.map_err(|e| PyErr::from(PyErrorWrapper::from(e)))
    }
}

/// Helper function to handle async errors
pub fn handle_async_error<T>(result: Result<T, Error>) -> PyResult<T> {
    result.into_py_result()
}

/// Run a closure and convert a Rust panic into a Python exception
///
/// Without this, a panic inside the extension module surfaces as
/// `pyo3_runtime.PanicException`, which callers do not expect and which leaves
/// any lock held during the panic poisoned. Catching it at the boundary leaves
/// an actionable `FerrocpError` instead.
///
/// Note: this only helps when the crate is built with unwinding panics. The
/// workspace release profile sets `panic = "abort"`, which makes any panic in a
/// release-built wheel abort the host interpreter; the Python extension must be
/// built with unwinding for this to apply.
pub fn catch_panic<T>(operation: &str, f: impl FnOnce() -> T) -> PyResult<T> {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match outcome {
        Ok(value) => Ok(value),
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            tracing::error!(
                "internal panic in '{}' was converted to a Python exception: {}",
                operation,
                message
            );
            Err(PyFerrocpError::new_err(format!(
                "internal error during '{operation}': {message}"
            )))
        }
    }
}

/// Extract a readable message from a caught panic payload
fn panic_message(payload: &(dyn std::any::Any + Send + 'static)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Full test on non-Windows platforms
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_error_conversion() {
        // `PyErr`'s `Display` impl renders the exception object through Python's
        // C API (it attaches internally), so an interpreter has to be running.
        // pyo3 no longer initialises one implicitly - the `auto-initialize`
        // feature was removed in 0.26 - so start it explicitly. `initialize()`
        // is idempotent and race-safe, so it does not matter which test in this
        // binary reaches it first.
        Python::initialize();

        let io_error = Error::Io {
            message: "Test IO error".to_string(),
            kind: None,
        };
        let py_err: PyErr = PyErrorWrapper::from(io_error).into();

        // Test that the error can be converted properly
        // Note: We can't easily test is_instance_of without a Python context
        assert!(py_err.to_string().contains("Test IO error"));
    }

    // Compilation-only test on Windows (avoids DLL dependency issues)
    #[cfg(target_os = "windows")]
    #[test]
    fn test_error_conversion_compilation() {
        // This test only verifies that the code compiles correctly on Windows
        // Runtime testing is skipped due to DLL dependency issues
        let io_error = Error::Io {
            message: "Test IO error".to_string(),
            kind: None,
        };
        let _wrapper = PyErrorWrapper::from(io_error);
        // Just verify compilation, don't run Python-specific code
    }
}
