use crate::error::{ErrorCode, ReplicashError};
use crate::tlv::types::{EventType, FieldType};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::{debug, error, trace};
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct TLVMessage {
    pub event_type: EventType,
    fields: SmallVec<[(FieldType, Bytes); 6]>,
}

impl TLVMessage {
    /// Create a new message manually (without raw)
    pub fn new(event_type: EventType) -> Self {
        debug!("Creating new TLVMessage with event type: {:?}", event_type);
        Self {
            event_type,
            fields: SmallVec::new(),
        }
    }

    /// Parse message from binary data
    pub fn parse(mut raw: Bytes) -> Result<Self, ReplicashError> {
        debug!("Parsing TLVMessage from {} bytes", raw.len());
        if raw.len() < 5 {
            error!("Message too short: {} bytes", raw.len());
            return Err(ReplicashError::new(
                ErrorCode::MalformedTlv,
                "Message too short",
            ));
        }

        // Read the length and event type
        let _length = raw.get_u32();
        let event_type = match EventType::from_u8(raw.get_u8()) {
            Ok(et) => {
                debug!("Parsed event type: {:?}", et);
                et
            },
            Err(e) => {
                error!("Invalid event type: {}", e);
                return Err(e);
            }
        };

        // Create a message
        let mut msg = Self::new(event_type);

        // Process all fields in the message
        let mut field_count = 0;
        while raw.remaining() >= 5 {
            let field_type = match FieldType::from_u8(raw.get_u8()) {
                Ok(ft) => {
                    trace!("Parsed field type: {:?}", ft);
                    ft
                },
                Err(e) => {
                    error!("Invalid field type: {}", e);
                    return Err(e);
                }
            };
            let field_len = raw.get_u32() as usize;
            trace!("Field length: {}", field_len);

            if raw.remaining() < field_len {
                error!("Incomplete TLV field: expected {} bytes, but only {} remaining", field_len, raw.remaining());
                return Err(ReplicashError::new(
                    ErrorCode::MalformedTlv,
                    "Incomplete TLV field",
                ));
            }
            // Add the field to the message
            msg.insert_field(field_type, raw.slice(..field_len));
            field_count += 1;

            // Skip over the field data
            raw.advance(field_len);
        }

        debug!("Successfully parsed TLVMessage with {} fields", field_count);
        Ok(msg)
    }

    /// Serializes the message as bytes
    /// 
    /// This implementation uses zero-copy techniques where possible to avoid
    /// unnecessary memory allocations and copies.
    pub fn encode(&self) -> Bytes {
        debug!("Encoding TLVMessage with event type: {:?}", self.event_type);
        // Calculate the total size needed for the message
        let mut total_size = 5; // 4 bytes for length + 1 byte for an event type

        // Calculate the size needed for all fields
        for (field_type, value) in &self.fields {
            total_size += 1 + 4 + value.len();
            trace!("Field {:?} adds {} bytes", field_type, 1 + 4 + value.len());
        }

        debug!("Total message size will be {} bytes", total_size);

        // Pre-allocate the entire buffer to avoid resizing
        let mut msg = BytesMut::with_capacity(total_size);

        // Write the header
        msg.put_u32(total_size as u32);
        msg.put_u8(self.event_type as u8);
        trace!("Wrote header: length={}, event_type={:?}", total_size, self.event_type);

        // Write all fields directly to the final buffer
        for (field_type, value) in &self.fields {
            msg.put_u8(*field_type as u8);
            msg.put_u32(value.len() as u32);
            msg.extend_from_slice(value);
            trace!("Wrote field: type={:?}, length={}", field_type, value.len());
        }

        debug!("Successfully encoded TLVMessage with {} fields", self.fields.len());
        msg.freeze()
    }

    /// Gets a field
    pub fn get_field(&self, field: FieldType) -> Option<&Bytes> {
        trace!("Getting field {:?} from message", field);
        let result = self.fields.iter().find_map(|(f, v)| {
            if *f == field {
                Some(v)
            } else {
                None
            }
        });

        if result.is_some() {
            trace!("Found field {:?} in message", field);
        } else {
            trace!("Field {:?} not found in message", field);
        }

        result
    }

    /// Sets or overwrites a field
    pub fn insert_field<V: Into<Bytes>>(&mut self, field: FieldType, value: V) {
        let value_bytes = value.into();
        trace!("Inserting field {:?} with {} bytes", field, value_bytes.len());

        for (f, v) in &mut self.fields {
            if *f == field {
                debug!("Overwriting existing field {:?}", field);
                *v = value_bytes;
                return;
            }
        }

        debug!("Adding new field {:?}", field);
        self.fields.push((field, value_bytes));
    }

    pub fn extract_event(buf: &[u8]) -> Result<EventType, ReplicashError> {
        debug!("Extracting event type from buffer of length {}", buf.len());
        if buf.len() < 5 {
            error!("Buffer too short for event extraction: {} bytes", buf.len());
            return Err(ReplicashError::new(ErrorCode::MalformedTlv, "Buffer too short"));
        }

        let _msg_len = match buf.get(0..4) {
            Some(len_bytes) => {
                let len = u32::from_be_bytes(len_bytes.try_into().unwrap());
                trace!("Message length: {}", len);
                len
            },
            None => {
                error!("Missing length field in buffer");
                return Err(ReplicashError::new(ErrorCode::MalformedTlv, "Missing length field"));
            }
        };

        let event_type = match EventType::from_u8(buf[4]) {
            Ok(et) => {
                debug!("Extracted event type: {:?}", et);
                et
            },
            Err(e) => {
                error!("Invalid event type byte: {}", buf[4]);
                return Err(e);
            }
        };
        Ok(event_type)
    }

    pub fn extract_field(buf: &[u8], target: FieldType) -> Result<&[u8], ReplicashError> {
        debug!("Extracting field {:?} from buffer of length {}", target, buf.len());
        if buf.len() < 5 {
            error!("Buffer too short for field extraction: {} bytes", buf.len());
            return Err(ReplicashError::new(ErrorCode::MalformedTlv, "Buffer too short"));
        }

        // Skip message length (4 bytes) and event type (1 byte)
        let _msg_len = match buf.get(0..4) {
            Some(len_bytes) => {
                let len = u32::from_be_bytes(len_bytes.try_into().unwrap());
                trace!("Message length: {}", len);
                len
            },
            None => {
                error!("Missing length field in buffer");
                return Err(ReplicashError::new(ErrorCode::MalformedTlv, "Missing length field"));
            }
        };

        let _event_type = buf[4];
        trace!("Event type byte: {}", _event_type);
        let mut offset = 5;

        while offset + 5 <= buf.len() {
            let field_type = match FieldType::from_u8(buf[offset]) {
                Ok(ft) => {
                    trace!("Found field type: {:?} at offset {}", ft, offset);
                    ft
                },
                Err(_) => {
                    error!("Invalid field type byte: {} at offset {}", buf[offset], offset);
                    return Err(ReplicashError::new(
                        ErrorCode::MalformedTlv,
                        "Invalid field type",
                    ));
                }
            };

            let field_len = match buf.get(offset + 1..offset + 5) {
                Some(len_bytes) => {
                    let len = u32::from_be_bytes(len_bytes.try_into().unwrap()) as usize;
                    trace!("Field length: {} at offset {}", len, offset + 1);
                    len
                },
                None => {
                    error!("Missing field length at offset {}", offset + 1);
                    return Err(ReplicashError::new(ErrorCode::MalformedTlv, "Missing field length"));
                }
            };

            offset += 5;
            if offset + field_len > buf.len() {
                error!("Field data too short: expected {} bytes at offset {}, but buffer only has {} bytes", 
                       field_len, offset, buf.len() - offset);
                return Err(ReplicashError::new(
                    ErrorCode::MalformedTlv,
                    "Field data too short",
                ));
            }

            if field_type == target {
                debug!("Found target field {:?} with length {} at offset {}", target, field_len, offset);
                return Ok(&buf[offset..offset + field_len]);
            }
            offset += field_len;
        }
        debug!("Target field {:?} not found in buffer", target);
        Ok(&[])
    }
}
