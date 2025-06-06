//! Error module for Replicash
//!
//! This module defines error types and codes used throughout the Replicash system.
//! Error codes are based on the specifications in the Fehlercodes.md document.

use thiserror::Error;
use std::fmt;
use crate::eventrelay::config;

/// Error code as defined in Fehlercodes.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Protocol errors (0x0001-0x0100)
    InvalidEventType = 0x0001,
    MalformedTlv = 0x0002,
    IncompleteMessage = 0x0003,
    UnsupportedVersion = 0x0004,

    // Application errors (0x0101-0x0200)
    KeyNotFound = 0x0101,
    TtlExpired = 0x0102,
    ValueTooLarge = 0x0103,
    DiskWriteFailed = 0x0104,

    // Cluster/State errors (0x0201-0x0300)
    ReadonlyMode = 0x0201,
    MasterUnavailable = 0x0202,
    SyncDenied = 0x0203,

    // System errors (0x0301-0x0400)
    InternalServerError = 0x0301,
    ConfigInvalid = 0x0302,

    // Client errors (0x0401-0x0500)
    ConnectionFailed = 0x0401,
    SendFailed = 0x0402,
    NotConnected = 0x0403,
}

impl ErrorCode {
    /// Get the error code as a u8
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Get the error code category
    pub fn category(&self) -> ErrorCategory {
        match *self as u8 {
            0x01..=0x10 => ErrorCategory::Protocol,
            0x11..=0x20 => ErrorCategory::Application,
            0x21..=0x30 => ErrorCategory::ClusterState,
            0x31..=0x40 => ErrorCategory::System,
            0x41..=0x50 => ErrorCategory::Client,
            _ => ErrorCategory::Unknown,
        }
    }

    /// Try to convert a u8 to an ErrorCode
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::InvalidEventType),
            0x02 => Some(Self::MalformedTlv),
            0x03 => Some(Self::IncompleteMessage),
            0x04 => Some(Self::UnsupportedVersion),
            0x11 => Some(Self::KeyNotFound),
            0x12 => Some(Self::TtlExpired),
            0x13 => Some(Self::ValueTooLarge),
            0x14 => Some(Self::DiskWriteFailed),
            0x21 => Some(Self::ReadonlyMode),
            0x22 => Some(Self::MasterUnavailable),
            0x23 => Some(Self::SyncDenied),
            0x31 => Some(Self::InternalServerError),
            0x32 => Some(Self::ConfigInvalid),
            0x41 => Some(Self::ConnectionFailed),
            0x42 => Some(Self::SendFailed),
            0x43 => Some(Self::NotConnected),
            _ => None,
        }
    }

    /// Try to convert a u16 to an ErrorCode
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            0x0001 => Some(Self::InvalidEventType),
            0x0002 => Some(Self::MalformedTlv),
            0x0003 => Some(Self::IncompleteMessage),
            0x0004 => Some(Self::UnsupportedVersion),
            0x0101 => Some(Self::KeyNotFound),
            0x0102 => Some(Self::TtlExpired),
            0x0103 => Some(Self::ValueTooLarge),
            0x0104 => Some(Self::DiskWriteFailed),
            0x0201 => Some(Self::ReadonlyMode),
            0x0202 => Some(Self::MasterUnavailable),
            0x0203 => Some(Self::SyncDenied),
            0x0301 => Some(Self::InternalServerError),
            0x0302 => Some(Self::ConfigInvalid),
            0x0401 => Some(Self::ConnectionFailed),
            0x0402 => Some(Self::SendFailed),
            0x0403 => Some(Self::NotConnected),
            _ => None,
        }
    }

    /// Get a human-readable description of the error code
    pub fn description(&self) -> &'static str {
        match self {
            Self::InvalidEventType => "Unknown or invalid event type",
            Self::MalformedTlv => "Malformed or inconsistent TLV field",
            Self::IncompleteMessage => "Incomplete or truncated message",
            Self::UnsupportedVersion => "Unsupported protocol version",
            Self::KeyNotFound => "Key not found in cache",
            Self::TtlExpired => "TTL expired for key",
            Self::ValueTooLarge => "Value size exceeds maximum",
            Self::DiskWriteFailed => "Failed to write to disk",
            Self::ReadonlyMode => "System is in read-only mode",
            Self::MasterUnavailable => "Master node is unavailable",
            Self::SyncDenied => "Synchronization denied",
            Self::InternalServerError => "Unexpected server error",
            Self::ConfigInvalid => "Invalid configuration",
            Self::ConnectionFailed => "Failed to connect to server",
            Self::SendFailed => "Failed to send message",
            Self::NotConnected => "Client is not connected",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::InvalidEventType => "INVALID_EVENT_TYPE",
            Self::MalformedTlv => "MALFORMED_TLV",
            Self::IncompleteMessage => "INCOMPLETE_MESSAGE",
            Self::UnsupportedVersion => "UNSUPPORTED_VERSION",
            Self::KeyNotFound => "KEY_NOT_FOUND",
            Self::TtlExpired => "TTL_EXPIRED",
            Self::ValueTooLarge => "VALUE_TOO_LARGE",
            Self::DiskWriteFailed => "DISK_WRITE_FAILED",
            Self::ReadonlyMode => "READONLY_MODE",
            Self::MasterUnavailable => "MASTER_UNAVAILABLE",
            Self::SyncDenied => "SYNC_DENIED",
            Self::InternalServerError => "INTERNAL_SERVER_ERROR",
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::ConnectionFailed => "CONNECTION_FAILED",
            Self::SendFailed => "SEND_FAILED",
            Self::NotConnected => "NOT_CONNECTED",
        };
        write!(f, "{} (0x{:04X})", name, *self as u8)
    }
}

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Protocol,
    Application,
    ClusterState,
    System,
    Client,
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol => write!(f, "Protocol"),
            Self::Application => write!(f, "Application"),
            Self::ClusterState => write!(f, "Cluster/State"),
            Self::System => write!(f, "System"),
            Self::Client => write!(f, "Client"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Main error type for Replicash
#[derive(Error, Debug)]
pub enum ReplicashError {
    #[error("{code}: {message}")]
    Standard {
        code: ErrorCode,
        message: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Other(String),
}

impl ReplicashError {
    /// Create a new standard error with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Standard {
            code,
            message: message.into(),
        }
    }

    /// Get the error code if this is a standard error
    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Standard { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Get the error message
    pub fn message(&self) -> String {
        match self {
            Self::Standard { message, .. } => message.clone(),
            _ => self.to_string(),
        }
    }

    /// Convert to a TLV error representation (for protocol use)
    pub fn to_tlv_error(&self) -> (u8, String) {
        match self {
            Self::Standard { code, message } => (code.as_u8(), message.clone()),
            _ => (ErrorCode::InternalServerError as u8, self.to_string()),
        }
    }
}

/// Result type alias for Replicash operations
pub type Result<T> = std::result::Result<T, ReplicashError>;

// Implement From<String> for ReplicashError
impl From<String> for ReplicashError {
    fn from(message: String) -> Self {
        Self::Other(message)
    }
}

// Implement From<&str> for ReplicashError
impl From<&str> for ReplicashError {
    fn from(message: &str) -> Self {
        Self::Other(message.to_string())
    }
}
