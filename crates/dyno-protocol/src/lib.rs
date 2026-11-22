//! Dyno network protocol.

/// Network protocol version. Increase it on every incompatible change.
/// Clients and servers with different versions refuse to connect.
pub const PROTOCOL_VERSION: u32 = 1;
