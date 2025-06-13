use std::sync::Arc;
use bytes::Bytes;
use log::{debug, error, info, warn};

use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::net::connection::Client;
use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};

pub async fn handle_sync_init(handler: Arc<EventHandler>, client: Arc<Client>, msg: &Bytes) {
    debug!("Received SyncInit message");

    // Extrahiere Peer-ID
    let id = match TLVMessage::extract_field(msg, FieldType::Key) {
        Ok(id) => id,
        Err(_) => {
            warn!("Missing peer ID in SyncInit message");
            let mut error_msg = TLVMessage::new(EventType::Error);
            error_msg.insert_field(FieldType::ErrorText, "Missing peer ID in SyncInit message");
            if let Err(e) = client.send_tlv(&error_msg) {
                error!("Failed to send error message: {}", e);
            }
            return;
        }
    };

    let id_bytes = Bytes::copy_from_slice(id);
    debug!("Extracted and using peer ID: {:?}", id_bytes);

    client.set_peer(true);
    debug!("Client is peer");

    // Unsubscribe from public topics since this is a peer
    handler.handle_unsubscribe_public(client.id());
    debug!("Unsubscribed peer from public topics");

    // Peer registrieren
    handler.clients.register_peer(id_bytes.clone(), client.clone());
    info!("Registered new peer with ID: {}", String::from_utf8_lossy(id));

    // SyncAck vorbereiten
    let mut ack = TLVMessage::new(EventType::SyncAck);
    ack.insert_field(FieldType::Key, id_bytes.clone());

    // Peer holen und Nachricht senden
    match handler.clients.get_peer(&id_bytes) {
        Some(peer) => {
            if let Err(e) = peer.send_tlv(&ack) {
                error!("Failed to send SyncAck to peer {}: {}", String::from_utf8_lossy(&id_bytes), e);
            } else {
                info!("SyncAck sent to peer: {}", String::from_utf8_lossy(&id_bytes));
            }
        }
        None => {
            error!("Peer not found after registration: {}", String::from_utf8_lossy(&id_bytes));
        }
    }
}

pub async fn handle_sync_ack(handler: Arc<EventHandler>, client: Arc<Client>, msg: &Bytes) {
    // Extrahiere Peer-ID
    let id = match TLVMessage::extract_field(msg, FieldType::Key) {
        Ok(id) => id,
        Err(_) => {
            warn!("Missing peer ID in SyncAck message");
            let mut error_msg = TLVMessage::new(EventType::Error);
            error_msg.insert_field(FieldType::ErrorText, "Missing peer ID in SyncAck message");
            if let Err(e) = client.send_tlv(&error_msg) {
                error!("Failed to send error message: {}", e);
            }
            return;
        }
    };

    let id_bytes = Bytes::copy_from_slice(id);
    debug!("Extracted and using peer ID: {:?}", id_bytes);

    client.set_peer(true);
    debug!("Client is peer");

    // Unsubscribe from public topics since this is a peer
    handler.handle_unsubscribe_public(client.id());
    debug!("Unsubscribed peer from public topics");

    // Peer registrieren
    handler.clients.register_peer(id_bytes.clone(), client.clone());
    info!("Registered new peer with ID: {}", String::from_utf8_lossy(id));
}

pub async fn handle_subscribe(handler: Arc<EventHandler>, client: Arc<Client>, msg: &Bytes) {
    if let Ok(topic) = TLVMessage::extract_field(msg, FieldType::Key) {
        handler.handle_subscribe(topic, client.id());
        info!("Client {:?} subscribed to topic", client.id());
    } else {
        warn!("Subscribe message missing topic field (Key)");
    }
}

pub async fn handle_unsubscribe(handler: Arc<EventHandler>, client: Arc<Client>, msg: &Bytes) {
    if let Ok(topic) = TLVMessage::extract_field(msg, FieldType::Key) {
        handler.handle_unsubscribe(topic, client.id());
        info!("Client {:?} unsubscribed from topic", client.id());
    } else {
        warn!("Unsubscribe message missing topic field (Key)");
    }
}

pub async fn handle_event(handler: Arc<EventHandler>, client: Arc<Client>, raw_msg: &Bytes) {
    // Parse the raw message into a TLVMessage
    match TLVMessage::parse(raw_msg.clone()) {
        Ok(msg) => {
            if let Some(topic) = msg.get_field(FieldType::Key) {
                if client.is_peer() {
                    handler.handle_event(topic, &msg, Some(client.id()), None)
                } else {
                    handler.handle_event(topic, &msg, None, Some(client.id()))
                }
            } else {
                warn!("Event message missing topic field (Key)");
            }
        },
        Err(e) => {
            error!("Failed to parse event message: {:?}", e);
        }
    }
}
