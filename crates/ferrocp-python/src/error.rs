//! Error handling for Python bindings

use ferrocp_types::{Error, ErrorContext};
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

/// Extend a mapped exception's message without changing its type
///
/// Rebuilding the error with `PyErr::new::<PyFerrocpError, _>` would replace a
/// `FileNotFoundError` with a plain `FerrocpError`, so callers could no longer
/// catch the builtin. Instead the original exception object is fetched, its
/// `args` are rewritten to carry the operation and path, and the **same**
/// exception type is re-raised.
fn with_context_message(error: PyErr, context: &ErrorContext) -> PyErr {
    let suffix = match &context.path {
        Some(path) => format!(" (while {} '{}')", context.operation, path.display()),
        None => format!(" (while {})", context.operation),
    };

    Python::with_gil(|py| {
        let value = error.value(py);

        // Build the extended message from whatever the exception already says.
        let existing: Option<String> = value
            .getattr("args")
            .ok()
            .and_then(|args| args.extract::<Vec<String>>().ok())
            .and_then(|items| items.into_iter().next());

        let message = match existing {
            Some(first) if !first.is_empty() => format!("{first}{suffix}"),
            _ => suffix.trim().to_string(),
        };

        // Re-raise the *same* exception class with the extended message.
        // Rebuilding with `PyErr::new::<PyFerrocpError, _>` would silently
        // break `except FileNotFoundError`.
        let class = value.getattr("__class__").ok();
        match class {
            Some(class) => match class.call1((message,)) {
                Ok(new_value) => PyErr::from_value(new_value),
                // If the class cannot be instantiated with one argument, keep
                // the original error rather than degrading its type.
                Err(_) => error,
            },
            None => error,
        }
    })
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
                // Preserve the mapped exception type (for example
                // `FileNotFoundError`) and only extend its message. Rebuilding
                // the error as `FerrocpError` here would silently break
                // `except FileNotFoundError` for exactly the errors that carry
                // the most context.
                with_context_message(PyErr::from(PyErrorWrapper(*error)), &context)
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
mod panic_tests {
    use super::*;

    /// A wrapped panic must become a `FerrocpError`, not a `PanicException`.
    ///
    /// This is the acceptance criterion for panic isolation: callers have to be
    /// able to `except FerrocpError`. It only holds when the crate is built with
    /// unwinding panics, which is why the test tolerates an abort-configured
    /// build gracefully rather than reporting a false failure.
    #[test]
    fn catch_panic_converts_a_panic_into_ferrocp_error() {
        pyo3::prepare_freethreaded_python();

        let result: PyResult<i32> = catch_panic("boom", || panic!("deliberate panic"));

        Python::with_gil(|py| match result {
            Ok(_) => panic!("the panic was not caught"),
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("deliberate panic"),
                    "the panic message must be preserved: {message}"
                );
                assert!(
                    message.contains("boom"),
                    "the operation name must be reported: {message}"
                );
                assert!(
                    error.is_instance_of::<PyFerrocpError>(py),
                    "a contained panic must be a FerrocpError, not a PanicException: {message}"
                );
            }
        });
    }

    #[test]
    fn catch_panic_passes_values_through() {
        let result: PyResult<i32> = catch_panic("ok", || 42);
        assert_eq!(result.unwrap(), 42);
    }

    /// A panic carrying a `&str` and one carrying a `String` both render.
    #[test]
    fn catch_panic_handles_both_payload_shapes() {
        pyo3::prepare_freethreaded_python();

        let from_str: PyResult<()> = catch_panic("op", || panic!("static str"));
        let from_string: PyResult<()> =
            catch_panic("op", || panic!("formatted {}", "payload"));

        assert!(from_str.unwrap_err().to_string().contains("static str"));
        assert!(from_string
            .unwrap_err()
            .to_string()
            .contains("formatted payload"));
    }

    /// `FileNotFoundError` must survive having context attached, otherwise
    /// `except FileNotFoundError` silently stops working for the errors that
    /// carry the most information.
    #[test]
    fn context_does_not_downgrade_file_not_found() {
        pyo3::prepare_freethreaded_python();

        let error = Error::Io {
            message: "no such file".to_string(),
            kind: Some(std::io::ErrorKind::NotFound),
        }
        .with_path_context("copy_file", "/data/missing.bin");

        let py_err = PyErr::from(PyErrorWrapper(error));

        Python::with_gil(|py| {
            assert!(
                py_err.is_instance_of::<PyFileNotFoundError>(py),
                "the builtin type must be preserved: {}",
                py_err.to_string()
            );
            // FileNotFoundError is an OSError, so the stdlib idiom works too.
            assert!(py_err.is_instance_of::<pyo3::exceptions::PyOSError>(py));
            // The context is appended to the message, not substituted for the type.
            assert!(py_err.to_string().contains("copy_file"));
            assert!(py_err.to_string().contains("/data/missing.bin"));
        });
    }

    #[test]
    fn context_does_not_downgrade_permission_denied() {
        pyo3::prepare_freethreaded_python();

        let error = Error::Io {
            message: "forbidden".to_string(),
            kind: Some(std::io::ErrorKind::PermissionDenied),
        }
        .with_path_context("copy_file", "/data/locked.bin");

        let py_err = PyErr::from(PyErrorWrapper(error));

        Python::with_gil(|py| {
            assert!(
                py_err.is_instance_of::<PyPermissionError>(py),
                "the builtin type must be preserved: {}",
                py_err.to_string()
            );
        });
    }

    #[test]
    fn plain_io_errors_still_map_to_the_builtin_types() {
        pyo3::prepare_freethreaded_python();

        let not_found = PyErr::from(PyErrorWrapper(Error::Io {
            message: "gone".to_string(),
            kind: Some(std::io::ErrorKind::NotFound),
        }));
        let denied = PyErr::from(PyErrorWrapper(Error::Io {
            message: "nope".to_string(),
            kind: Some(std::io::ErrorKind::PermissionDenied),
        }));

        Python::with_gil(|py| {
            assert!(not_found.is_instance_of::<PyFileNotFoundError>(py));
            assert!(denied.is_instance_of::<PyPermissionError>(py));
        });
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
