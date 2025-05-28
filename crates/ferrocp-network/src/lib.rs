//! Modern network communication for FerroCP
//!
//! This crate provides high-performance network communication capabilities for FerroCP with features like:
//!
//! - **QUIC/HTTP3 Protocol**: Modern, secure, and fast network protocol
//! - **Network File Transfer**: Efficient file transfer over network with compression
//! - **Resume Support**: Automatic resume of interrupted transfers
//! - **Connection Pooling**: Efficient connection management and reuse
//! - **Progress Tracking**: Real-time progress reporting for network operations
//! - **Error Recovery**: Robust error handling with automatic retry mechanisms
//!
//! # Examples
//!
//! ```rust
//! use ferrocp_network::{NetworkClient, NetworkServer, TransferRequest};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Start a network server
//! let mut server = NetworkServer::new("127.0.0.1:8080").await?;
//! server.start().await?;
//!
//! // Create a client and transfer a file
//! let mut client = NetworkClient::new().await?;
//! let request = TransferRequest::new("source.txt", "destination.txt");
//! let result = client.transfer_file("127.0.0.1:8080", request).await?;
//! println!("Transferred {} bytes", result.bytes_transferred);
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod client;
pub mod connection;
pub mod protocol;
pub mod resume;
pub mod server;
pub mod transfer;

pub use client::{ClientConfig, NetworkClient};
pub use connection::{ConnectionManager, ConnectionPool};
pub use protocol::{NetworkProtocol, ProtocolVersion};
pub use resume::{ResumeInfo, ResumeManager};
pub use server::{NetworkServer, ServerConfig};
pub use transfer::{TransferProgress, TransferRequest, TransferResult};
