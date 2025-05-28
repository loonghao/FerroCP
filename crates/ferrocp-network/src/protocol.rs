//! Network protocol definitions and implementations

use ferrocp_types::NetworkProtocol as ConfigNetworkProtocol;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Network protocol types supported by FerroCP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkProtocol {
    /// QUIC protocol (default)
    Quic,
    /// HTTP/3 over QUIC
    Http3,
    /// HTTP/2
    Http2,
    /// Plain TCP (for testing only)
    Tcp,
}

impl Default for NetworkProtocol {
    fn default() -> Self {
        Self::Quic
    }
}

impl fmt::Display for NetworkProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quic => write!(f, "QUIC"),
            Self::Http3 => write!(f, "HTTP/3"),
            Self::Http2 => write!(f, "HTTP/2"),
            Self::Tcp => write!(f, "TCP"),
        }
    }
}

impl From<ConfigNetworkProtocol> for NetworkProtocol {
    fn from(config_protocol: ConfigNetworkProtocol) -> Self {
        match config_protocol {
            ConfigNetworkProtocol::Quic => Self::Quic,
            ConfigNetworkProtocol::Http3 => Self::Http3,
            ConfigNetworkProtocol::Http2 => Self::Http2,
            ConfigNetworkProtocol::Tcp => Self::Tcp,
        }
    }
}

/// Protocol version information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
}

impl ProtocolVersion {
    /// Create a new protocol version
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Current protocol version
    pub const fn current() -> Self {
        Self::new(1, 0, 0)
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major && self.minor <= other.minor
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Protocol message types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Handshake message
    Handshake,
    /// File transfer request
    TransferRequest,
    /// File transfer response
    TransferResponse,
    /// File data chunk
    DataChunk,
    /// Transfer progress update
    Progress,
    /// Error message
    Error,
    /// Heartbeat/keepalive
    Heartbeat,
    /// Transfer completion
    Complete,
    /// Resume request
    Resume,
}

/// Protocol message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Message type
    pub message_type: MessageType,
    /// Message ID for correlation
    pub message_id: uuid::Uuid,
    /// Protocol version
    pub version: ProtocolVersion,
    /// Message payload
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

impl ProtocolMessage {
    /// Create a new protocol message
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            message_id: uuid::Uuid::new_v4(),
            version: ProtocolVersion::current(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

/// Handshake information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeInfo {
    /// Client/server identifier
    pub peer_id: String,
    /// Supported protocols
    pub supported_protocols: Vec<NetworkProtocol>,
    /// Protocol version
    pub version: ProtocolVersion,
    /// Capabilities
    pub capabilities: Vec<String>,
}

impl HandshakeInfo {
    /// Create a new handshake info
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            supported_protocols: vec![
                NetworkProtocol::Quic,
                NetworkProtocol::Http3,
                NetworkProtocol::Http2,
            ],
            version: ProtocolVersion::current(),
            capabilities: vec![
                "compression".to_string(),
                "resume".to_string(),
                "progress".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_compatibility() {
        let v1_0_0 = ProtocolVersion::new(1, 0, 0);
        let v1_1_0 = ProtocolVersion::new(1, 1, 0);
        let v2_0_0 = ProtocolVersion::new(2, 0, 0);

        assert!(v1_0_0.is_compatible_with(&v1_1_0));
        assert!(!v1_1_0.is_compatible_with(&v1_0_0));
        assert!(!v1_0_0.is_compatible_with(&v2_0_0));
    }

    #[test]
    fn test_protocol_message_serialization() {
        let message = ProtocolMessage::new(MessageType::Handshake, b"test payload".to_vec());

        let bytes = message.to_bytes().unwrap();
        let deserialized = ProtocolMessage::from_bytes(&bytes).unwrap();

        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.payload, deserialized.payload);
    }

    #[test]
    fn test_handshake_info() {
        let handshake = HandshakeInfo::new("test-client".to_string());

        assert_eq!(handshake.peer_id, "test-client");
        assert!(!handshake.supported_protocols.is_empty());
        assert!(!handshake.capabilities.is_empty());
    }
}
