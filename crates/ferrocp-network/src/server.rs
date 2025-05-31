//! Network server implementation for FerroCP

use crate::{
    protocol::{HandshakeInfo, MessageType, NetworkProtocol, ProtocolMessage},
    transfer::TransferProgress,
};
use ferrocp_config::NetworkConfig;
use ferrocp_types::{Error, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

/// Network server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_addr: SocketAddr,
    /// Network protocol to use
    pub protocol: NetworkProtocol,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u8,
    /// Working directory for file operations
    pub working_dir: std::path::PathBuf,
    /// Enable file caching
    pub enable_caching: bool,
    /// Cache directory
    pub cache_dir: std::path::PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            protocol: NetworkProtocol::Quic,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(300), // 5 minutes
            enable_compression: true,
            compression_level: 6,
            working_dir: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            enable_caching: true,
            cache_dir: std::path::PathBuf::from(".ferrocp_cache"),
        }
    }
}

impl From<NetworkConfig> for ServerConfig {
    fn from(network_config: NetworkConfig) -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(), // Default, should be configurable
            protocol: network_config.protocol.into(),
            max_connections: network_config.max_connections as usize,
            connection_timeout: network_config.timeouts.connect_timeout,
            request_timeout: network_config
                .timeouts
                .operation_timeout
                .unwrap_or(Duration::from_secs(300)),
            enable_compression: true,
            compression_level: 6,
            working_dir: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            enable_caching: network_config.enable_connection_pooling, // Reuse this flag
            cache_dir: std::path::PathBuf::from(".ferrocp_cache"),
        }
    }
}

/// Client session information
#[derive(Debug, Clone)]
pub struct ClientSession {
    /// Session ID
    pub id: uuid::Uuid,
    /// Client address
    pub client_addr: SocketAddr,
    /// Handshake information
    pub handshake_info: Option<HandshakeInfo>,
    /// Active transfers
    pub active_transfers: HashMap<uuid::Uuid, TransferProgress>,
    /// Session start time
    pub started_at: std::time::Instant,
    /// Last activity time
    pub last_activity: std::time::Instant,
}

impl ClientSession {
    /// Create a new client session
    pub fn new(client_addr: SocketAddr) -> Self {
        let now = std::time::Instant::now();
        Self {
            id: uuid::Uuid::new_v4(),
            client_addr,
            handshake_info: None,
            active_transfers: HashMap::new(),
            started_at: now,
            last_activity: now,
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get idle time
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Network server for handling file transfer requests
#[derive(Debug)]
pub struct NetworkServer {
    config: ServerConfig,
    sessions: Arc<RwLock<HashMap<uuid::Uuid, ClientSession>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    is_running: Arc<RwLock<bool>>,
}

impl NetworkServer {
    /// Create a new network server
    pub async fn new(bind_addr: &str) -> Result<Self> {
        let addr: SocketAddr = bind_addr.parse().map_err(|e| Error::Network {
            message: format!("Invalid bind address '{}': {}", bind_addr, e),
        })?;

        let config = ServerConfig {
            bind_addr: addr,
            ..Default::default()
        };

        Self::with_config(config).await
    }

    /// Create a new network server with custom configuration
    pub async fn with_config(config: ServerConfig) -> Result<Self> {
        // Create cache directory if enabled
        if config.enable_caching && !config.cache_dir.exists() {
            tokio::fs::create_dir_all(&config.cache_dir)
                .await
                .map_err(|e| Error::Io {
                    message: format!(
                        "Failed to create cache directory '{}': {}",
                        config.cache_dir.display(),
                        e
                    ),
                })?;
        }

        Ok(Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the server
    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write().await = true;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        info!(
            "Starting FerroCP network server on {}",
            self.config.bind_addr
        );

        // Start the server based on protocol
        match self.config.protocol {
            NetworkProtocol::Quic | NetworkProtocol::Http3 => {
                self.start_quic_server(&mut shutdown_rx).await
            }
            NetworkProtocol::Http2 | NetworkProtocol::Tcp => {
                self.start_tcp_server(&mut shutdown_rx).await
            }
        }
    }

    /// Stop the server
    pub async fn stop(&mut self) -> Result<()> {
        *self.is_running.write().await = false;

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Close all active sessions
        let mut sessions = self.sessions.write().await;
        sessions.clear();

        info!("FerroCP network server stopped");
        Ok(())
    }

    /// Start QUIC server
    async fn start_quic_server(&self, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
        // TODO: Implement QUIC server
        info!("QUIC server would start here on {}", self.config.bind_addr);

        // Wait for shutdown signal
        shutdown_rx.recv().await;
        Ok(())
    }

    /// Start TCP server
    async fn start_tcp_server(&self, shutdown_rx: &mut mpsc::Receiver<()>) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(self.config.bind_addr)
            .await
            .map_err(|e| Error::Network {
                message: format!("Failed to bind to {}: {}", self.config.bind_addr, e),
            })?;

        info!("TCP server listening on {}", self.config.bind_addr);

        let sessions = Arc::clone(&self.sessions);
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);

        tokio::select! {
            _ = async {
                loop {
                    if !*is_running.read().await {
                        break;
                    }

                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            let sessions = Arc::clone(&sessions);
                            let config = config.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_client_connection(stream, addr, sessions, config).await {
                                    error!("Error handling client {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
            } => {},
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal");
            }
        }

        Ok(())
    }

    /// Handle client connection
    async fn handle_client_connection(
        mut stream: tokio::net::TcpStream,
        client_addr: SocketAddr,
        sessions: Arc<RwLock<HashMap<uuid::Uuid, ClientSession>>>,
        config: ServerConfig,
    ) -> Result<()> {
        info!("New client connection from {}", client_addr);

        // Create client session
        let mut session = ClientSession::new(client_addr);
        let session_id = session.id;

        // Store session
        sessions.write().await.insert(session_id, session.clone());

        // Handle client messages
        let result = Self::handle_client_messages(&mut stream, &mut session, &config).await;

        // Clean up session
        sessions.write().await.remove(&session_id);

        match result {
            Ok(()) => info!("Client {} disconnected normally", client_addr),
            Err(e) => error!("Client {} disconnected with error: {}", client_addr, e),
        }

        Ok(())
    }

    /// Handle client messages
    async fn handle_client_messages(
        stream: &mut tokio::net::TcpStream,
        session: &mut ClientSession,
        config: &ServerConfig,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        loop {
            // Read message length
            let mut len_buf = [0u8; 4];
            match stream.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("Client disconnected");
                    break;
                }
                Err(e) => {
                    return Err(Error::Network {
                        message: format!("Failed to read message length: {}", e),
                    })
                }
            }

            let message_len = u32::from_be_bytes(len_buf) as usize;
            if message_len > 1024 * 1024 {
                // 1MB limit
                return Err(Error::Network {
                    message: "Message too large".to_string(),
                });
            }

            // Read message data
            let mut message_buf = vec![0u8; message_len];
            stream
                .read_exact(&mut message_buf)
                .await
                .map_err(|e| Error::Network {
                    message: format!("Failed to read message data: {}", e),
                })?;

            // Parse message
            let message =
                ProtocolMessage::from_bytes(&message_buf).map_err(|e| Error::Network {
                    message: format!("Failed to parse message: {}", e),
                })?;

            session.update_activity();

            // Handle message
            let response = Self::handle_message(message, session, config).await?;

            // Send response
            let response_data = response.to_bytes().map_err(|e| Error::Network {
                message: format!("Failed to serialize response: {}", e),
            })?;

            let response_len = (response_data.len() as u32).to_be_bytes();
            stream
                .write_all(&response_len)
                .await
                .map_err(|e| Error::Network {
                    message: format!("Failed to write response length: {}", e),
                })?;

            stream
                .write_all(&response_data)
                .await
                .map_err(|e| Error::Network {
                    message: format!("Failed to write response data: {}", e),
                })?;
        }

        Ok(())
    }

    /// Handle a protocol message
    async fn handle_message(
        message: ProtocolMessage,
        session: &mut ClientSession,
        config: &ServerConfig,
    ) -> Result<ProtocolMessage> {
        match message.message_type {
            MessageType::Handshake => Self::handle_handshake(message, session).await,
            MessageType::TransferRequest => {
                Self::handle_transfer_request(message, session, config).await
            }
            MessageType::Resume => Self::handle_resume_request(message, session, config).await,
            MessageType::Heartbeat => Self::handle_heartbeat(message).await,
            _ => Err(Error::Network {
                message: format!("Unsupported message type: {:?}", message.message_type),
            }),
        }
    }

    /// Handle handshake message
    async fn handle_handshake(
        message: ProtocolMessage,
        session: &mut ClientSession,
    ) -> Result<ProtocolMessage> {
        let client_handshake: HandshakeInfo =
            bincode::deserialize(&message.payload).map_err(|e| Error::Network {
                message: format!("Failed to deserialize handshake: {}", e),
            })?;

        debug!(
            "Received handshake from client: {}",
            client_handshake.peer_id
        );

        // Store handshake info
        session.handshake_info = Some(client_handshake);

        // Create server handshake response
        let server_handshake = HandshakeInfo::new("ferrocp-server".to_string());
        let response_data = bincode::serialize(&server_handshake).map_err(|e| Error::Network {
            message: format!("Failed to serialize handshake response: {}", e),
        })?;

        Ok(ProtocolMessage::new(MessageType::Handshake, response_data))
    }

    /// Handle transfer request
    async fn handle_transfer_request(
        _message: ProtocolMessage,
        session: &mut ClientSession,
        _config: &ServerConfig,
    ) -> Result<ProtocolMessage> {
        // TODO: Implement transfer request handling
        debug!(
            "Received transfer request from client {}",
            session.client_addr
        );

        // For now, just send a success response
        let response_data = b"Transfer request accepted".to_vec();
        Ok(ProtocolMessage::new(
            MessageType::TransferResponse,
            response_data,
        ))
    }

    /// Handle resume request
    async fn handle_resume_request(
        _message: ProtocolMessage,
        session: &mut ClientSession,
        _config: &ServerConfig,
    ) -> Result<ProtocolMessage> {
        // TODO: Implement resume request handling
        debug!(
            "Received resume request from client {}",
            session.client_addr
        );

        // For now, just send a success response
        let response_data = b"Resume request accepted".to_vec();
        Ok(ProtocolMessage::new(
            MessageType::TransferResponse,
            response_data,
        ))
    }

    /// Handle heartbeat message
    async fn handle_heartbeat(message: ProtocolMessage) -> Result<ProtocolMessage> {
        // Echo back the heartbeat
        Ok(ProtocolMessage::new(
            MessageType::Heartbeat,
            message.payload,
        ))
    }

    /// Get server statistics
    pub async fn stats(&self) -> ServerStats {
        let sessions = self.sessions.read().await;
        let active_sessions = sessions.len();
        let total_transfers: usize = sessions.values().map(|s| s.active_transfers.len()).sum();

        ServerStats {
            active_sessions,
            total_transfers,
            is_running: *self.is_running.read().await,
            bind_addr: self.config.bind_addr,
            protocol: self.config.protocol,
        }
    }
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    /// Number of active client sessions
    pub active_sessions: usize,
    /// Total number of active transfers
    pub total_transfers: usize,
    /// Whether the server is running
    pub is_running: bool,
    /// Server bind address
    pub bind_addr: SocketAddr,
    /// Protocol used
    pub protocol: NetworkProtocol,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_session_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let session = ClientSession::new(addr);

        assert_eq!(session.client_addr, addr);
        assert!(session.handshake_info.is_none());
        assert!(session.active_transfers.is_empty());
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = NetworkServer::new("127.0.0.1:0").await.unwrap();
        let stats = server.stats().await;

        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_transfers, 0);
        assert!(!stats.is_running);
    }
}
