//! Network connection management and pooling

use crate::protocol::{NetworkProtocol, ProtocolMessage};
use ferrocp_types::{Error, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info};

/// Connection identifier
pub type ConnectionId = uuid::Uuid;

/// Network connection trait
#[async_trait::async_trait]
pub trait NetworkConnection: Send + Sync {
    /// Send a message
    async fn send(&mut self, message: ProtocolMessage) -> Result<()>;

    /// Receive a message
    async fn receive(&mut self) -> Result<ProtocolMessage>;

    /// Close the connection
    async fn close(&mut self) -> Result<()>;

    /// Check if connection is alive
    fn is_alive(&self) -> bool;

    /// Get connection info
    fn info(&self) -> ConnectionInfo;
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Connection ID
    pub id: ConnectionId,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Local address
    pub local_addr: SocketAddr,
    /// Protocol used
    pub protocol: NetworkProtocol,
    /// Connection established time
    pub established_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(
        id: ConnectionId,
        remote_addr: SocketAddr,
        local_addr: SocketAddr,
        protocol: NetworkProtocol,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            remote_addr,
            local_addr,
            protocol,
            established_at: now,
            last_activity: now,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Add sent bytes
    pub fn add_sent_bytes(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.update_activity();
    }

    /// Add received bytes
    pub fn add_received_bytes(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.update_activity();
    }

    /// Get connection age
    pub fn age(&self) -> Duration {
        self.established_at.elapsed()
    }

    /// Get idle time
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections per endpoint
    pub max_connections_per_endpoint: usize,
    /// Maximum total connections
    pub max_total_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Connection keep-alive interval
    pub keep_alive_interval: Duration,
    /// Enable connection reuse
    pub enable_reuse: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_endpoint: 5,
            max_total_connections: 50,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            keep_alive_interval: Duration::from_secs(30),
            enable_reuse: true,
        }
    }
}

/// Connection pool for managing network connections
pub struct ConnectionPool {
    config: PoolConfig,
    connections: Arc<RwLock<HashMap<SocketAddr, Vec<Arc<Mutex<dyn NetworkConnection>>>>>>,
    connection_info: Arc<RwLock<HashMap<ConnectionId, ConnectionInfo>>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a connection to the specified endpoint
    pub async fn get_connection(
        &self,
        endpoint: SocketAddr,
    ) -> Result<Arc<Mutex<dyn NetworkConnection>>> {
        if self.config.enable_reuse {
            // Try to reuse an existing connection
            if let Some(connection) = self.try_reuse_connection(endpoint).await? {
                return Ok(connection);
            }
        }

        // Create a new connection
        self.create_connection(endpoint).await
    }

    /// Try to reuse an existing connection
    async fn try_reuse_connection(
        &self,
        endpoint: SocketAddr,
    ) -> Result<Option<Arc<Mutex<dyn NetworkConnection>>>> {
        let mut connections = self.connections.write().await;

        if let Some(endpoint_connections) = connections.get_mut(&endpoint) {
            // Find a healthy connection
            let i = 0;
            while i < endpoint_connections.len() {
                let connection = &endpoint_connections[i];
                let is_alive = {
                    let conn = connection.lock().await;
                    conn.is_alive()
                };

                if is_alive {
                    // Move the connection to the end (LRU)
                    let connection = endpoint_connections.remove(i);
                    endpoint_connections.push(connection.clone());
                    debug!("Reusing connection to {}", endpoint);
                    return Ok(Some(connection));
                } else {
                    // Remove dead connection
                    let dead_connection = endpoint_connections.remove(i);
                    let conn_id = {
                        let conn = dead_connection.lock().await;
                        conn.info().id
                    };
                    self.connection_info.write().await.remove(&conn_id);
                    debug!("Removed dead connection to {}", endpoint);
                }
            }
        }

        Ok(None)
    }

    /// Create a new connection
    async fn create_connection(
        &self,
        endpoint: SocketAddr,
    ) -> Result<Arc<Mutex<dyn NetworkConnection>>> {
        // Check connection limits
        self.check_connection_limits(endpoint).await?;

        // For now, return an error since we haven't implemented actual connections yet
        Err(Error::Network {
            message: "Connection creation not implemented yet".to_string(),
        })
    }

    /// Check connection limits
    async fn check_connection_limits(&self, endpoint: SocketAddr) -> Result<()> {
        let connections = self.connections.read().await;

        // Check per-endpoint limit
        if let Some(endpoint_connections) = connections.get(&endpoint) {
            if endpoint_connections.len() >= self.config.max_connections_per_endpoint {
                return Err(Error::Network {
                    message: format!(
                        "Maximum connections per endpoint ({}) exceeded for {}",
                        self.config.max_connections_per_endpoint, endpoint
                    ),
                });
            }
        }

        // Check total limit
        let total_connections: usize = connections.values().map(|v| v.len()).sum();
        if total_connections >= self.config.max_total_connections {
            return Err(Error::Network {
                message: format!(
                    "Maximum total connections ({}) exceeded",
                    self.config.max_total_connections
                ),
            });
        }

        Ok(())
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        let mut connection_info = self.connection_info.write().await;
        let mut removed_count = 0;

        for (endpoint, endpoint_connections) in connections.iter_mut() {
            let mut i = 0;
            while i < endpoint_connections.len() {
                let should_remove = {
                    let conn = endpoint_connections[i].lock().await;
                    let info = conn.info();
                    !conn.is_alive() || info.idle_time() > self.config.idle_timeout
                };

                if should_remove {
                    let removed_connection = endpoint_connections.remove(i);
                    let conn_id = {
                        let conn = removed_connection.lock().await;
                        conn.info().id
                    };
                    connection_info.remove(&conn_id);
                    removed_count += 1;
                    debug!("Removed idle connection to {}", endpoint);
                } else {
                    i += 1;
                }
            }
        }

        // Remove empty endpoint entries
        connections.retain(|_, v| !v.is_empty());

        if removed_count > 0 {
            info!("Cleaned up {} idle connections", removed_count);
        }

        Ok(())
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let connections = self.connections.read().await;
        let connection_info = self.connection_info.read().await;

        let total_connections: usize = connections.values().map(|v| v.len()).sum();
        let endpoints_count = connections.len();

        PoolStats {
            total_connections,
            endpoints_count,
            max_connections_per_endpoint: self.config.max_connections_per_endpoint,
            max_total_connections: self.config.max_total_connections,
            active_connections: connection_info.len(),
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections
    pub total_connections: usize,
    /// Number of endpoints
    pub endpoints_count: usize,
    /// Maximum connections per endpoint
    pub max_connections_per_endpoint: usize,
    /// Maximum total connections
    pub max_total_connections: usize,
    /// Number of active connections
    pub active_connections: usize,
}

/// Connection manager for handling connection lifecycle
pub struct ConnectionManager {
    pool: ConnectionPool,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pool: ConnectionPool::new(config),
        }
    }

    /// Get a connection to the specified endpoint
    pub async fn connect(&self, endpoint: SocketAddr) -> Result<Arc<Mutex<dyn NetworkConnection>>> {
        self.pool.get_connection(endpoint).await
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) -> Result<()> {
        let pool = self.pool.clone();
        let cleanup_interval = pool.config.keep_alive_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                if let Err(e) = pool.cleanup_idle_connections().await {
                    error!("Failed to cleanup idle connections: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Get connection pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.pool.stats().await
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connections: Arc::clone(&self.connections),
            connection_info: Arc::clone(&self.connection_info),
        }
    }
}
