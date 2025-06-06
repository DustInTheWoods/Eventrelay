use log::{debug, error};
use crate::error::{ErrorCode, ReplicashError};

/// Event types as defined in the TLV Protocol V1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    // Commands
    Set = 0x01,
    Get = 0x02,
    Delete = 0x03,
    GetGroup = 0x04,

    // Events
    Event = 0x10,
    Subscribe = 0x11,

    // Sync
    SyncPush = 0x20,
    SyncPull = 0x21,
    SyncInit = 0x22,
    SyncAck = 0x23,
    SyncSummary = 0x24,
    SyncTombstone = 0x25,

    // Responses
    Error = 0xF0,
    Ok = 0xF1,
}

impl EventType {
    /// Convert a u8 to an EventType
    pub fn from_u8(value: u8) -> Result<Self, ReplicashError> {
        // Debug: Log the raw value being converted
        debug!("🔄 Converting u8 to EventType: value = 0x{:02X}", value);

        match value {
            0x01 => {
                Ok(Self::Set)
            },
            0x02 => {
                Ok(Self::Get)
            },
            0x03 => {
                Ok(Self::Delete)
            },
            0x04 => {
                Ok(Self::GetGroup)
            },
            0x10 => {
                Ok(Self::Event)
            },
            0x11 => {
                Ok(Self::Subscribe)
            },
            0x20 => {
                Ok(Self::SyncPush)
            },
            0x21 => {
                Ok(Self::SyncPull)
            },
            0x22 => {
                Ok(Self::SyncInit)
            },
            0x23 => {
                Ok(Self::SyncAck)
            },
            0x24 => {
                Ok(Self::SyncSummary)
            },
            0x25 => {
                Ok(Self::SyncTombstone)
            },
            0xF0 => {
                Ok(Self::Error)
            },
            0xF1 => {
                Ok(Self::Ok)
            },
            _ => {
                error!("Unknown event type: 0x{:02X}", value);
                Err(ReplicashError::new(
                    ErrorCode::InvalidEventType,
                    format!("Unknown event type: 0x{:02X}", value),
                ))
            },
        }
    }
}


/// Field types as defined in the TLV Protocol V1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    Key = 0x01,
    Value = 0x02,
    Ttl = 0x03,
    Persistent = 0x04,
    Group = 0x05,
    Timestamp = 0x06,
    Version = 0x07,
    ErrorCode = 0x08,
    ErrorText = 0x09,
}

impl FieldType {
    /// Convert a u8 to a FieldType
    pub fn from_u8(value: u8) -> Result<Self, ReplicashError> {
        match value {
            0x01 => Ok(Self::Key),
            0x02 => Ok(Self::Value),
            0x03 => Ok(Self::Ttl),
            0x04 => Ok(Self::Persistent),
            0x05 => Ok(Self::Group),
            0x06 => Ok(Self::Timestamp),
            0x07 => Ok(Self::Version),
            0x08 => Ok(Self::ErrorCode),
            0x09 => Ok(Self::ErrorText),
            _ => Err(ReplicashError::new(
                ErrorCode::MalformedTlv,
                format!("Unknown field type: 0x{:02X}", value),
            )),
        }
    }
}
