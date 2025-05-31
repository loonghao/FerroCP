//! Network client implementation for FerroCP

use crate::{
    connection::{ConnectionManager, PoolConfig},
    protocol::{HandshakeInfo, MessageType, NetworkProtocol, ProtocolMessage},
    resume::{ResumeConfig, ResumeManager},
    transfer::{TransferProgress, TransferRequest, TransferResult},
};
use ferrocp_config::NetworkConfig;
use ferrocp_types::{Error, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Network client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Network protocol to use
    pub protocol: NetworkProtocol,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u8,
    /// Connection pool configuration
    pub pool_config: PoolConfig,
    /// Resume configuration
    pub resume_config: ResumeConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            protocol: NetworkProtocol::Quic,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(300), // 5 minutes
            max_retries: 3,
            enable_compression: true,
            compression_level: 6,
            pool_config: PoolConfig::default(),
            resume_config: ResumeConfig::default(),
        }
    }
}

impl From<NetworkConfig> for ClientConfig {
    fn from(network_config: NetworkConfig) -> Self {
        Self {
            protocol: network_config.protocol.into(),
            connect_timeout: network_config.timeouts.connect_timeout,
            request_timeout: network_config
                .timeouts
                .operation_timeout
                .unwrap_or(Duration::from_secs(300)),
            max_retries: network_config.retry.max_retries,
            enable_compression: true,
            compression_level: 6,
            pool_config: PoolConfig {
                max_connections_per_endpoint: network_config.max_connections as usize,
                enable_reuse: network_config.enable_connection_pooling,
                ..Default::default()
            },
            resume_config: ResumeConfig::default(),
        }
    }
}

/// Network client for file transfers
pub struct NetworkClient {
    config: ClientConfig,
    connection_manager: ConnectionManager,
    resume_manager: Arc<ResumeManager>,
    active_transfers: Arc<RwLock<std::collections::HashMap<uuid::Uuid, TransferProgress>>>,
}

impl NetworkClient {
    /// Create a new network client
    pub async fn new() -> Result<Self> {
        Self::with_config(ClientConfig::default()).await
    }

    /// Create a new network client with custom configuration
    pub async fn with_config(config: ClientConfig) -> Result<Self> {
        let connection_manager = ConnectionManager::new(config.pool_config.clone());
        let resume_manager = Arc::new(ResumeManager::new(config.resume_config.clone()).await?);

        // Start connection cleanup task
        connection_manager.start_cleanup_task().await?;

        Ok(Self {
            config,
            connection_manager,
            resume_manager,
            active_transfers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Transfer a file to a remote server
    pub async fn transfer_file(
        &mut self,
        server_addr: &str,
        request: TransferRequest,
    ) -> Result<TransferResult> {
        let endpoint: SocketAddr = server_addr.parse().map_err(|e| Error::Network {
            message: format!("Invalid server address '{}': {}", server_addr, e),
        })?;

        // Check if transfer can be resumed
        let resume_info = if self.config.resume_config.max_retries > 0 {
            self.resume_manager
                .load_resume_info(&request.request_id)
                .await?
        } else {
            None
        };

        if let Some(mut resume_info) = resume_info {
            if self.resume_manager.can_resume(&request.request_id).await {
                info!("Resuming transfer for request {}", request.request_id);
                resume_info.increment_retry();
                return self.resume_transfer(endpoint, request, resume_info).await;
            }
        }

        // Start new transfer
        self.start_new_transfer(endpoint, request).await
    }

    /// Start a new file transfer
    async fn start_new_transfer(
        &mut self,
        endpoint: SocketAddr,
        request: TransferRequest,
    ) -> Result<TransferResult> {
        let _start_time = Instant::now();

        // Get connection
        let connection = self.connection_manager.connect(endpoint).await?;

        // Perform handshake
        self.perform_handshake(&connection).await?;

        // Send transfer request
        let file_size = self.get_file_size(&request.source).await?;
        let mut progress =
            TransferProgress::new(request.request_id, file_size, request.options.chunk_size);

        // Store active transfer
        self.active_transfers
            .write()
            .await
            .insert(request.request_id, progress.clone());

        // Send request message
        let request_message = self.create_transfer_request_message(&request, file_size)?;
        {
            let mut conn = connection.lock().await;
            conn.send(request_message).await?;
        }

        // Wait for response
        let response = {
            let mut conn = connection.lock().await;
            conn.receive().await?
        };

        if response.message_type != MessageType::TransferResponse {
            return Err(Error::Network {
                message: "Invalid response to transfer request".to_string(),
            });
        }

        // Start file transfer
        let result = self
            .transfer_file_chunks(connection, &request, &mut progress)
            .await;

        // Clean up active transfer
        self.active_transfers
            .write()
            .await
            .remove(&request.request_id);

        match result {
            Ok(stats) => {
                // Remove resume info on successful completion
                let _ = self
                    .resume_manager
                    .remove_resume_info(&request.request_id)
                    .await;

                Ok(TransferResult::new(request.request_id, stats))
            }
            Err(e) => {
                // Save resume info on failure
                if request.options.enable_resume {
                    let resume_info = crate::resume::ResumeInfo::new(&request, &progress, None);
                    let _ = self.resume_manager.save_resume_info(resume_info).await;
                }
                Err(e)
            }
        }
    }

    /// Resume an interrupted transfer
    async fn resume_transfer(
        &mut self,
        endpoint: SocketAddr,
        request: TransferRequest,
        resume_info: crate::resume::ResumeInfo,
    ) -> Result<TransferResult> {
        info!(
            "Resuming transfer from {} bytes ({}%)",
            resume_info.bytes_transferred,
            resume_info.completion_percentage()
        );

        // Get connection
        let connection = self.connection_manager.connect(endpoint).await?;

        // Perform handshake
        self.perform_handshake(&connection).await?;

        // Create progress from resume info
        let mut progress = TransferProgress::new(
            request.request_id,
            resume_info.total_size,
            request.options.chunk_size,
        );
        progress.update(resume_info.bytes_transferred, 0.0);
        progress.update_chunk(resume_info.last_chunk_sequence);

        // Store active transfer
        self.active_transfers
            .write()
            .await
            .insert(request.request_id, progress.clone());

        // Send resume request
        let resume_message = self.create_resume_request_message(&request, &resume_info)?;
        {
            let mut conn = connection.lock().await;
            conn.send(resume_message).await?;
        }

        // Wait for response
        let response = {
            let mut conn = connection.lock().await;
            conn.receive().await?
        };

        if response.message_type != MessageType::TransferResponse {
            return Err(Error::Network {
                message: "Invalid response to resume request".to_string(),
            });
        }

        // Continue file transfer from where it left off
        let result = self
            .transfer_file_chunks(connection, &request, &mut progress)
            .await;

        // Clean up active transfer
        self.active_transfers
            .write()
            .await
            .remove(&request.request_id);

        match result {
            Ok(stats) => {
                // Remove resume info on successful completion
                let _ = self
                    .resume_manager
                    .remove_resume_info(&request.request_id)
                    .await;

                let mut transfer_result = TransferResult::new(request.request_id, stats);
                transfer_result = transfer_result.with_resume_info(true, resume_info.retry_count);
                Ok(transfer_result)
            }
            Err(e) => {
                // Update resume info on failure
                if request.options.enable_resume {
                    let mut updated_resume_info =
                        crate::resume::ResumeInfo::new(&request, &progress, None);
                    updated_resume_info.retry_count = resume_info.retry_count + 1;
                    let _ = self
                        .resume_manager
                        .save_resume_info(updated_resume_info)
                        .await;
                }
                Err(e)
            }
        }
    }

    /// Perform handshake with server
    async fn perform_handshake(
        &self,
        connection: &Arc<tokio::sync::Mutex<dyn crate::connection::NetworkConnection>>,
    ) -> Result<()> {
        let handshake_info = HandshakeInfo::new("ferrocp-client".to_string());
        let handshake_data = bincode::serialize(&handshake_info).map_err(|e| Error::Network {
            message: format!("Failed to serialize handshake: {}", e),
        })?;

        let handshake_message = ProtocolMessage::new(MessageType::Handshake, handshake_data);

        {
            let mut conn = connection.lock().await;
            conn.send(handshake_message).await?;
        }

        // Wait for handshake response
        let response = {
            let mut conn = connection.lock().await;
            conn.receive().await?
        };

        if response.message_type != MessageType::Handshake {
            return Err(Error::Network {
                message: "Invalid handshake response".to_string(),
            });
        }

        debug!("Handshake completed successfully");
        Ok(())
    }

    /// Get file size
    async fn get_file_size(&self, path: &std::path::Path) -> Result<u64> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| Error::Io {
            message: format!(
                "Failed to get file metadata for '{}': {}",
                path.display(),
                e
            ),
        })?;

        Ok(metadata.len())
    }

    /// Create transfer request message
    fn create_transfer_request_message(
        &self,
        request: &TransferRequest,
        file_size: u64,
    ) -> Result<ProtocolMessage> {
        #[derive(serde::Serialize)]
        struct TransferRequestData {
            request_id: uuid::Uuid,
            source: std::path::PathBuf,
            destination: std::path::PathBuf,
            file_size: u64,
            options: crate::transfer::TransferOptions,
        }

        let request_data = TransferRequestData {
            request_id: request.request_id,
            source: request.source.clone(),
            destination: request.destination.clone(),
            file_size,
            options: request.options.clone(),
        };

        let data = bincode::serialize(&request_data).map_err(|e| Error::Network {
            message: format!("Failed to serialize transfer request: {}", e),
        })?;

        Ok(ProtocolMessage::new(MessageType::TransferRequest, data))
    }

    /// Create resume request message
    fn create_resume_request_message(
        &self,
        _request: &TransferRequest,
        resume_info: &crate::resume::ResumeInfo,
    ) -> Result<ProtocolMessage> {
        let data = bincode::serialize(resume_info).map_err(|e| Error::Network {
            message: format!("Failed to serialize resume request: {}", e),
        })?;

        Ok(ProtocolMessage::new(MessageType::Resume, data))
    }

    /// Transfer file chunks (placeholder implementation)
    async fn transfer_file_chunks(
        &self,
        _connection: Arc<tokio::sync::Mutex<dyn crate::connection::NetworkConnection>>,
        _request: &TransferRequest,
        _progress: &mut TransferProgress,
    ) -> Result<ferrocp_types::CopyStats> {
        // TODO: Implement actual file chunk transfer
        Err(Error::Network {
            message: "File chunk transfer not implemented".to_string(),
        })
    }

    /// Get active transfers
    pub async fn get_active_transfers(&self) -> Vec<TransferProgress> {
        self.active_transfers
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Cancel a transfer
    pub async fn cancel_transfer(&self, request_id: &uuid::Uuid) -> Result<()> {
        if self
            .active_transfers
            .write()
            .await
            .remove(request_id)
            .is_some()
        {
            info!("Cancelled transfer {}", request_id);
            Ok(())
        } else {
            Err(Error::Network {
                message: format!("Transfer {} not found", request_id),
            })
        }
    }
}
