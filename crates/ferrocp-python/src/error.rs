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
}

pub use exceptions::*;

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
            Error::Io { message } => PyIoError::new_err(message),
            Error::FileNotFound { path } => {
                PyIoError::new_err(format!("File not found: {}", path.display()))
            }
            Error::PermissionDenied { path } => {
                PyIoError::new_err(format!("Permission denied: {}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_error = Error::Io {
            message: "Test IO error".to_string(),
        };
        let py_err: PyErr = PyErrorWrapper::from(io_error).into();

        // Test that the error can be converted properly
        // Note: We can't easily test is_instance_of without a Python context
        assert!(py_err.to_string().contains("Test IO error"));
    }
}
