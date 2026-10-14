//! Network functionality for Python bindings

use crate::config::PyNetworkConfig;
use crate::copy::PyCopyResult;
use crate::error::IntoPyResult;
use crate::progress::{ProgressCallback, PyProgress};
use ferrocp_network::{ClientConfig, NetworkClient, TransferRequest, TransferResult};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

/// Python wrapper for network transfer results
#[pyclass(name = "TransferResult", from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTransferResult {
    /// Total bytes transferred
    #[pyo3(get)]
    pub bytes_transferred: u64,
    /// Transfer duration in seconds
    #[pyo3(get)]
    pub duration_seconds: f64,
    /// Average transfer speed (bytes per second)
    #[pyo3(get)]
    pub average_speed: f64,
    /// Whether the transfer was resumed
    #[pyo3(get)]
    pub was_resumed: bool,
    /// Number of retry attempts
    #[pyo3(get)]
    pub retry_count: u32,
    /// Whether the operation was successful
    #[pyo3(get)]
    pub success: bool,
    /// Error message if operation failed
    #[pyo3(get)]
    pub error_message: Option<String>,
}

#[pymethods]
impl PyTransferResult {
    /// Create a new transfer result
    #[new]
    pub fn new() -> Self {
        Self {
            bytes_transferred: 0,
            duration_seconds: 0.0,
            average_speed: 0.0,
            was_resumed: false,
            retry_count: 0,
            success: false,
            error_message: None,
        }
    }

    /// Get formatted transfer speed
    pub fn format_speed(&self) -> String {
        format_bytes_per_second(self.average_speed)
    }

    /// Get formatted duration
    pub fn format_duration(&self) -> String {
        format_duration_seconds(self.duration_seconds)
    }

    /// String representation
    fn __str__(&self) -> String {
        if self.success {
            format!(
                "TransferResult(success=True, bytes={}, speed={}, duration={})",
                format_bytes(self.bytes_transferred),
                self.format_speed(),
                self.format_duration()
            )
        } else {
            format!(
                "TransferResult(success=False, error={})",
                self.error_message.as_deref().unwrap_or("Unknown error")
            )
        }
    }
}

impl Default for PyTransferResult {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TransferResult> for PyTransferResult {
    fn from(result: TransferResult) -> Self {
        Self {
            bytes_transferred: result.bytes_transferred,
            duration_seconds: result.duration.as_secs_f64(),
            average_speed: result.average_speed,
            was_resumed: result.was_resumed,
            retry_count: result.retry_count,
            success: true,
            error_message: None,
        }
    }
}

/// Python wrapper for network client
#[pyclass(name = "NetworkClient", skip_from_py_object)]
pub struct PyNetworkClient {
    client: Option<NetworkClient>,
}

#[pymethods]
impl PyNetworkClient {
    /// Create a new network client
    #[new]
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Initialize the network client with configuration
    #[pyo3(signature = (config = None))]
    pub fn initialize<'py>(
        &mut self,
        py: Python<'py>,
        config: Option<PyNetworkConfig>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Initialize synchronously to avoid borrowing issues
        let client = if let Some(config) = config {
            let protocol = match config.to_network_protocol() {
                ferrocp_types::NetworkProtocol::Quic => ferrocp_network::NetworkProtocol::Quic,
                ferrocp_types::NetworkProtocol::Http3 => ferrocp_network::NetworkProtocol::Http3,
                ferrocp_types::NetworkProtocol::Http2 => ferrocp_network::NetworkProtocol::Http2,
                ferrocp_types::NetworkProtocol::Tcp => ferrocp_network::NetworkProtocol::Tcp,
            };
            let client_config = ClientConfig {
                protocol,
                max_retries: config.max_retries,
                connect_timeout: std::time::Duration::from_secs_f64(config.connect_timeout),
                request_timeout: std::time::Duration::from_secs_f64(config.operation_timeout),
                ..Default::default()
            };
            pyo3_async_runtimes::tokio::get_runtime()
                .block_on(async { NetworkClient::with_config(client_config).await })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
        } else {
            pyo3_async_runtimes::tokio::get_runtime()
                .block_on(async { NetworkClient::new().await })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
        };

        self.client = Some(client);

        future_into_py(py, async move { Ok(()) })
    }

    /// Transfer a file over network
    #[pyo3(signature = (_server_addr, source, destination, _progress_callback = None))]
    pub fn transfer_file<'py>(
        &mut self,
        py: Python<'py>,
        _server_addr: String,
        source: String,
        destination: String,
        _progress_callback: Option<ProgressCallback>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _source_path = PathBuf::from(source);
        let _dest_path = PathBuf::from(destination);

        // Initialize client if not already done
        if self.client.is_none() {
            let client = pyo3_async_runtimes::tokio::get_runtime()
                .block_on(async { NetworkClient::new().await })
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            self.client = Some(client);
        }

        // For now, return a simple result since we can't access self.client in async move
        // TODO: Implement proper async network transfer
        future_into_py(py, async move {
            // Simulate transfer operation
            let mut result = PyTransferResult::new();
            result.success = true;
            result.bytes_transferred = 0; // Placeholder
            Ok(result)
        })
    }

    /// Get active transfers
    pub fn get_active_transfers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let has_client = self.client.is_some();
        future_into_py(py, async move {
            if has_client {
                // For now, return empty list since we can't access self.client in async move
                // TODO: Implement proper async access to client
                Ok(Vec::<PyProgress>::new())
            } else {
                Ok(Vec::<PyProgress>::new())
            }
        })
    }

    /// Cancel a transfer
    pub fn cancel_transfer<'py>(
        &self,
        py: Python<'py>,
        transfer_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let has_client = self.client.is_some();
        future_into_py(py, async move {
            if has_client {
                let _uuid = uuid::Uuid::parse_str(&transfer_id)
                    .map_err(|e| ferrocp_types::Error::Network {
                        message: format!("Invalid transfer ID: {}", e),
                    })
                    .into_py_result()?;

                // TODO: Implement proper async access to client for cancellation
                Ok(false) // Return false for now since we can't access client
            } else {
                Ok(false)
            }
        })
    }
}

/// Default port used when `server` carries no explicit port.
const DEFAULT_SERVER_PORT: u16 = 8080;

/// Resolve a `server` value into a concrete `SocketAddr`.
///
/// `ferrocp_network::NetworkClient::transfer_file` parses its address with
/// `SocketAddr::from_str`, which accepts only an IP literal plus a port and does
/// no name resolution. `server` is documented as a host name or IP address and
/// the `ferrocp copy-with-server --server HOST` CLI asks users for a host name,
/// so resolve the name here before handing the address down.
///
/// A `server` value that already carries a port (including an IPv6 literal in
/// brackets) keeps that port; `port` applies only when `server` has none.
fn resolve_server_addr(server: &str, port: u16) -> Result<SocketAddr, String> {
    let server = server.trim();
    if server.is_empty() {
        return Err(
            "invalid server address: the value is empty; expected a host name, an IPv4 \
             literal or a bracketed IPv6 literal, each optionally followed by :port"
                .to_string(),
        );
    }

    // `[::1]:8080` / `127.0.0.1:9000` — the caller was explicit, use it as-is.
    if let Ok(addr) = server.parse::<SocketAddr>() {
        return Ok(addr);
    }

    // Otherwise resolve the name. A bare IPv6 literal such as `::1` parses as
    // neither `SocketAddr` nor a host:port pair, but `ToSocketAddrs` handles it,
    // so only the "no explicit port" case gets the port appended.
    let candidate = match split_host_port(server) {
        Some(_) => server.to_string(),
        None => format!("{server}:{port}"),
    };

    let mut addr = candidate.to_socket_addrs().map_err(|e| {
        format!(
            "invalid server address '{server}': {e}; expected a host name, an IPv4 literal or a \
             bracketed IPv6 literal, each optionally followed by :port"
        )
    })?;

    addr.next().ok_or_else(|| {
        format!("invalid server address '{server}': it does not resolve to any socket address")
    })
}

/// Split `value` into host and port when it already carries a port.
///
/// `None` means "no explicit port": the value is a bare host name, an IPv4
/// literal, a bracketed IPv6 literal, or a bare IPv6 literal such as `::1`. Appending
/// `:<port>` to any of those yields input `ToSocketAddrs` accepts, which is why a
/// bare IPv6 literal correctly reports `None` here.
fn split_host_port(value: &str) -> Option<(&str, &str)> {
    if let Some(rest) = value.strip_prefix('[') {
        // Bracketed IPv6: the port, if any, follows the closing bracket.
        let (host, tail) = rest.split_once(']')?;
        let port = tail.strip_prefix(':')?;
        return Some((host, port));
    }

    let (host, port) = value.rsplit_once(':')?;
    // A trailing colon (`example.com:`) is an empty port, and a value with two
    // colons is a bare IPv6 literal rather than a host:port pair.
    if host.is_empty() || port.is_empty() || host.contains(':') {
        return None;
    }
    Some((host, port))
}

/// Python wrapper for the network backed copy engine
///
/// `EACopy.copy_with_server` historically reached for a `NetworkCopyEngine` that
/// was never registered on the extension module, so the call raised
/// `ImportError` instead of transferring anything. This class closes that gap by
/// driving `ferrocp_network::NetworkClient` directly.
#[pyclass(name = "NetworkCopyEngine", skip_from_py_object)]
pub struct PyNetworkCopyEngine;

#[pymethods]
impl PyNetworkCopyEngine {
    /// Create a new network copy engine
    #[new]
    pub fn new() -> Self {
        Self
    }

    /// Copy `source` to `destination` through a FerroCP server
    ///
    /// `server` is the host name or IP address of the ferrocp-network server,
    /// optionally followed by `:port`; `port` is used when `server` carries no
    /// port of its own. The transfer runs to completion on the shared Tokio
    /// runtime, so the call blocks the calling Python thread until the transfer
    /// finishes or fails. The GIL is released for the duration of the transfer,
    /// so other Python threads keep running while it is in flight.
    #[pyo3(signature = (source, destination, server, port = DEFAULT_SERVER_PORT))]
    pub fn copy_with_server(
        &self,
        py: Python<'_>,
        source: String,
        destination: String,
        server: String,
        port: u16,
    ) -> PyResult<PyCopyResult> {
        let server_addr =
            resolve_server_addr(&server, port).map_err(pyo3::exceptions::PyValueError::new_err)?;
        let request = TransferRequest::new(&source, &destination);

        let result = py.detach(|| {
            pyo3_async_runtimes::tokio::get_runtime().block_on(async {
                let mut client = NetworkClient::new().await.into_py_result()?;
                client
                    .transfer_file(&server_addr.to_string(), request)
                    .await
                    .into_py_result()
            })
        })?;

        Ok(PyCopyResult {
            bytes_copied: result.bytes_transferred,
            files_copied: result.stats.files_copied,
            duration_seconds: result.duration.as_secs_f64(),
            transfer_rate: result.average_speed,
            success: true,
            error_message: None,
        })
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format bytes per second as human-readable string
fn format_bytes_per_second(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut size = bytes_per_sec;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration in seconds as human-readable string
fn format_duration_seconds(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_result_creation() {
        let result = PyTransferResult::new();
        assert_eq!(result.bytes_transferred, 0);
        assert_eq!(result.duration_seconds, 0.0);
        assert!(!result.success);
    }

    #[test]
    fn test_network_client_creation() {
        let client = PyNetworkClient::new();
        assert!(client.client.is_none());
    }

    #[test]
    fn test_format_functions() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes_per_second(1024.0), "1.0 KB/s");
        assert_eq!(format_duration_seconds(65.0), "1m 5s");
    }

    #[test]
    fn test_resolve_server_addr_ipv4_literal_uses_port() {
        let addr = resolve_server_addr("127.0.0.1", 9000).unwrap();
        assert_eq!(addr, "127.0.0.1:9000".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_server_addr_ipv4_literal_with_explicit_port_keeps_it() {
        let addr = resolve_server_addr("127.0.0.1:1234", 9000).unwrap();
        assert_eq!(addr, "127.0.0.1:1234".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_server_addr_localhost_hostname_resolves() {
        // The contract under test: a host name, not just an IP literal, has to
        // produce a SocketAddr. `localhost` resolves on every supported target.
        let addr = resolve_server_addr("localhost", 8080).unwrap();
        assert_eq!(addr.port(), 8080);
        assert!(addr.ip().is_loopback());
    }

    #[test]
    fn test_resolve_server_addr_localhost_with_explicit_port() {
        let addr = resolve_server_addr("localhost:4321", 8080).unwrap();
        assert_eq!(addr.port(), 4321);
        assert!(addr.ip().is_loopback());
    }

    #[test]
    fn test_resolve_server_addr_bracketed_ipv6() {
        let addr = resolve_server_addr("[::1]", 9000).unwrap();
        assert_eq!(addr, "[::1]:9000".parse::<SocketAddr>().unwrap());

        let addr = resolve_server_addr("[::1]:7000", 9000).unwrap();
        assert_eq!(addr, "[::1]:7000".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_server_addr_bare_ipv6_literal() {
        let addr = resolve_server_addr("::1", 9000).unwrap();
        assert_eq!(addr, "[::1]:9000".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_server_addr_trims_whitespace() {
        let addr = resolve_server_addr("  127.0.0.1  ", 9100).unwrap();
        assert_eq!(addr, "127.0.0.1:9100".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_resolve_server_addr_empty_is_diagnosable() {
        let err = resolve_server_addr("   ", 8080).unwrap_err();
        assert!(
            err.contains("empty"),
            "expected an 'empty' diagnosis, got: {err}"
        );
    }

    #[test]
    fn test_resolve_server_addr_unresolvable_host_names_the_host() {
        // An unresolvable name must report which name failed, not a generic
        // "Invalid server address" with no context.
        let err = resolve_server_addr("no-such-host.invalid", 8080).unwrap_err();
        assert!(
            err.contains("no-such-host.invalid"),
            "expected the failing host in the message, got: {err}"
        );
        assert!(
            err.contains("host name"),
            "expected the message to state the accepted forms, got: {err}"
        );
    }

    #[test]
    fn test_resolve_server_addr_rejects_out_of_range_port() {
        let err = resolve_server_addr("127.0.0.1:99999", 8080).unwrap_err();
        assert!(
            err.contains("127.0.0.1:99999"),
            "expected the offending address in the message, got: {err}"
        );
    }

    #[test]
    fn test_split_host_port() {
        assert_eq!(
            split_host_port("example.com:80"),
            Some(("example.com", "80"))
        );
        assert_eq!(split_host_port("127.0.0.1:80"), Some(("127.0.0.1", "80")));
        assert_eq!(split_host_port("[::1]:80"), Some(("::1", "80")));
        assert_eq!(split_host_port("example.com"), None);
        assert_eq!(split_host_port("[::1]"), None);
        assert_eq!(split_host_port("::1"), None);
        assert_eq!(split_host_port("example.com:"), None);
    }

    #[test]
    fn test_default_server_port_matches_signature_default() {
        // `copy_with_server`'s `port` default is 8080; keep the constant in step
        // so a change to one is caught here rather than at the call site.
        assert_eq!(DEFAULT_SERVER_PORT, 8080);
    }
}
