use std::collections::HashMap;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::debug;
use crate::error::{ErrorCode, ReplicashError};
use crate::tlv::types::{EventType, FieldType};

#[derive(Debug, Clone)]
pub struct TLVMessage {
    pub event_type: EventType,
    raw: Bytes,
    fields: HashMap<FieldType, Bytes>
}

impl TLVMessage {
    /// Neue Nachricht von Hand erstellen (ohne raw)
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            raw: Bytes::new(),
            fields: HashMap::new(),
        }
    }

    /// Nachricht aus binären Daten parsen
    pub fn parse(mut raw: Bytes) -> Result<Self, ReplicashError> {
        if raw.len() < 5 {
            return Err(ReplicashError::new(
                ErrorCode::MalformedTlv,
                "Message too short",
            ));
        }

        // Debug: Log the first few bytes of the message
        debug!("📦 Parsing message: first bytes = {:?}", &raw[..std::cmp::min(10, raw.len())]);

        // Store a copy of the original raw data for slicing
        let original_raw = raw.clone();

        // Skip the length field (4 bytes) before reading the event type
        let _length = raw.get_u32();
        debug!("📏 Message length = {}", _length);

        let event_type = EventType::from_u8(raw.get_u8())?;

        let mut fields = HashMap::new();

        // Track our position in the original buffer
        let mut offset = 5; // 4 bytes for length + 1 byte for event type

        while raw.remaining() >= 5 {
            let field_type = FieldType::from_u8(raw.get_u8())?;
            offset += 1;

            let field_len = raw.get_u32() as usize;
            offset += 4;

            if raw.remaining() < field_len {
                return Err(ReplicashError::new(
                    ErrorCode::MalformedTlv,
                    "Incomplete TLV field",
                ));
            }

            // Create a slice of the original raw data for this field without copying
            let field_slice = original_raw.slice(offset..(offset + field_len));
            fields.insert(field_type, field_slice);

            // Skip over the field data
            raw.advance(field_len);
            offset += field_len;
        }

        Ok(Self {
            event_type,
            raw,
            fields,
        })
    }

    /// Serialisiert die Nachricht als Bytes
    pub fn encode(&self) -> Bytes {
        // Calculate the total size needed for the message
        let mut total_size = 5; // 4 bytes for length + 1 byte for event type

        // Calculate size needed for all fields
        debug!("📊 Encoding message with fields:");
        for (field_type, value) in &self.fields {
            let field_size = 1 + 4 + value.len(); // 1 byte for field type + 4 bytes for length + field data
            debug!("  - Field {:?}: {} bytes", field_type, value.len());
            total_size += field_size;
        }
        debug!("📏 Total calculated message size: {} bytes", total_size);

        // Check if the message size exceeds the maximum allowed size
        const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
        if total_size > MAX_MESSAGE_SIZE {
            debug!("⚠️ Message too large: {} bytes (Maximum: {} bytes)", total_size, MAX_MESSAGE_SIZE);
            // Return an empty message to prevent sending a message that's too large
            return Bytes::new();
        }

        // Pre-allocate the entire buffer to avoid resizing
        let mut msg = BytesMut::with_capacity(total_size);

        // Write the header
        debug!("📝 Writing message length to header: {} bytes", total_size);
        msg.put_u32(total_size as u32);
        msg.put_u8(self.event_type as u8);

        // Debug: Print the first few bytes of the header
        if msg.len() >= 5 {
            debug!("📊 Message header bytes: [0x{:02X} 0x{:02X} 0x{:02X} 0x{:02X} 0x{:02X}]",
                msg[0], msg[1], msg[2], msg[3], msg[4]);
            // Also print the length as a u32
            let len_bytes = [msg[0], msg[1], msg[2], msg[3]];
            let len = u32::from_be_bytes(len_bytes);
            debug!("📏 Length from header: {} bytes", len);
        }

        // Write all fields directly to the final buffer
        for (field_type, value) in &self.fields {
            msg.put_u8(*field_type as u8);
            msg.put_u32(value.len() as u32);
            // Use extend_from_slice which is optimized for Bytes
            msg.extend_from_slice(value);
        }

        // Debug: Log the event type and the first few bytes of the encoded message
        debug!("📤 Encoding message: event_type = {:?}, first bytes = {:?}", 
               self.event_type, 
               &msg[..std::cmp::min(10, msg.len())]);

        // Debug: Log the entire message if it's not too large
        if msg.len() <= 100 {
            debug!("📦 Full encoded message: {:?}", &msg[..]);
        } else {
            debug!("📦 Encoded message too large to log fully: {} bytes", msg.len());
            // Log the first 50 bytes
            debug!("📦 First 50 bytes: {:?}", &msg[..50]);
            // Log the last 50 bytes
            debug!("📦 Last 50 bytes: {:?}", &msg[(msg.len() - 50)..]);
        }

        msg.freeze()
    }

    /// Holt ein Feld
    pub fn get_field(&self, field: FieldType) -> Option<&Bytes> {
        self.fields.get(&field)
    }

    /// Setzt oder überschreibt ein Feld
    pub fn insert_field<V: Into<Bytes>>(&mut self, field: FieldType, value: V) {
        self.fields.insert(field, value.into());
    }
}
