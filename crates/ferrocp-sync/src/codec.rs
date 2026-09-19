//! Binary codec used for FerroCP sync payloads and persisted state.
//!
//! Every caller must go through this module so the layout stays consistent:
//! switching to another configuration silently breaks the wire protocol and any
//! cache or resume file written by an earlier version.
//!
//! The layout is pinned with [`bincode::config::legacy()`], which is byte-for-byte
//! identical to the bincode 1.x default encoding (little endian, fixed-size
//! integer encoding, no size limit). That keeps the format unchanged across the
//! bincode 1.x to 2.x migration.

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Error returned when a value cannot be encoded.
pub type EncodeError = bincode::error::EncodeError;

/// Error returned when a value cannot be decoded.
pub type DecodeError = bincode::error::DecodeError;

/// Encode `value` into the pinned FerroCP binary layout.
///
/// # Errors
///
/// Returns [`EncodeError`] if `value` cannot be represented in the layout.
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, EncodeError>
where
    T: Serialize,
{
    bincode::serde::encode_to_vec(value, bincode::config::legacy())
}

/// Decode a `T` from the pinned FerroCP binary layout.
///
/// Trailing bytes after the decoded value are ignored, which keeps the call
/// sites compatible with framed transports that may over-read.
///
/// # Errors
///
/// Returns [`DecodeError`] if `bytes` is not a valid encoding of `T`.
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, DecodeError>
where
    T: DeserializeOwned,
{
    bincode::serde::decode_from_slice(bytes, bincode::config::legacy()).map(|(value, _)| value)
}
