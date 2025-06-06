use std::collections::HashMap;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::{debug, trace, warn};
use crate::error::{ErrorCode, ReplicashError};
use crate::tlv::types::{EventType, FieldType};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

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

        // Trace: look at the first few bytes of the message
        trace!("📦 Parsing message start: {:?}", &raw[..std::cmp::min(10, raw.len())]);

        // Store a copy of the original raw data for slicing
        let original_raw = raw.clone();

        // Skip the length field (4 bytes) before reading the event type
        let _length = raw.get_u32();
        trace!("📏 Message length = {}", _length);

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

        debug!("Parsed message {:?} with {} fields", event_type, fields.len());

        Ok(Self {
            event_type,
            raw,
            fields,
        })
    }

    /// Berechnet die Gesamtgröße der serialisierten Nachricht
    fn total_size(&self) -> usize {
        let mut size = 5; // 4 bytes length + 1 byte event type
        for value in self.fields.values() {
            size += 1 + 4 + value.len();
        }
        size
    }

    /// Serialisiert die Nachricht als Bytes
    pub fn encode(&self) -> Bytes {
        // Calculate the total size needed for the message
        let total_size = self.total_size();
        debug!("Encoding {:?} with {} fields ({} bytes)", self.event_type, self.fields.len(), total_size);

        // Check if the message size exceeds the maximum allowed size
        if total_size > MAX_MESSAGE_SIZE {
            warn!("Message too large: {} bytes (Maximum: {} bytes)", total_size, MAX_MESSAGE_SIZE);
            return Bytes::new();
        }

        // Pre-allocate the entire buffer to avoid resizing
        let mut msg = BytesMut::with_capacity(total_size);

        // Write the header
        msg.put_u32(total_size as u32);
        msg.put_u8(self.event_type as u8);
        trace!("Header written for {:?}", self.event_type);

        // Write all fields directly to the final buffer
        for (field_type, value) in &self.fields {
            msg.put_u8(*field_type as u8);
            msg.put_u32(value.len() as u32);
            // Use extend_from_slice which is optimized for Bytes
            msg.extend_from_slice(value);
        }

        trace!("Encoded message {:?} size: {} bytes", self.event_type, msg.len());

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
