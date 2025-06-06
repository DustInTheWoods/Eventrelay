use std::sync::Arc;
use bytes::Bytes;
use log::{info, warn, error, debug, trace};
use rand::RngCore;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::io::split;

use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};
use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::broadcaster::PeerSender;
use crate::eventrelay::client::ClientSender;
use crate::eventrelay::types::ClientId;

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

fn process_initial_message(
    msg: TLVMessage,
    writer: &Arc<tokio::sync::Mutex<WriteHalf<TcpStream>>>,
    handler: &Arc<EventHandler>,
    client_id: ClientId,
) -> Option<String> {
    match msg.event_type {
        EventType::SyncInit => {
            if let Some(raw) = msg.get_field(FieldType::Key) {
                if let Ok(id_str) = std::str::from_utf8(raw) {
                    let id = id_str.to_string();

                    let peer_sender = Arc::new(WriterPeerSender { writer: writer.clone() });
                    handler.peers.register(id.clone(), peer_sender);

                    let mut ack = TLVMessage::new(EventType::SyncAck);
                    ack.insert_field(FieldType::Key, Bytes::copy_from_slice(id.as_bytes()));
                    handler.peers.get(&id).map(|s| s.send(&ack));
                    info!("✅ SyncAck sent to {}", id);

                    return Some(id);
                }
            }
        }
        EventType::SyncAck => {
            if let Some(raw) = msg.get_field(FieldType::Key) {
                if let Ok(id_str) = std::str::from_utf8(raw) {
                    let id = id_str.to_string();

                    if !handler.peers.contains(&id) {
                        let sender = Arc::new(WriterPeerSender { writer: writer.clone() });
                        handler.peers.register(id.clone(), sender);
                    }

                    info!("🔄 SyncAck received from {}", id);
                    return Some(id);
                }
            }
        }
        EventType::Subscribe => {
            if let Some(topic) = msg.get_field(FieldType::Key) {
                handler.handle_subscribe(topic, client_id);
            }
        }
        _ => {
            handler.handle_event(&msg, None, Some(client_id));
        }
    }

    None
}

/// PeerSender
pub struct WriterPeerSender {
    writer: Arc<tokio::sync::Mutex<WriteHalf<TcpStream>>>,
}

impl PeerSender for WriterPeerSender {
    fn send(&self, msg: &TLVMessage) {
        let data = msg.encode();
        let writer = self.writer.clone();

        tokio::spawn(async move {
            match writer.try_lock() {
                Ok(mut w) => {
                    if let Err(e) = w.write_all(&data).await {
                        error!("Peer send error: {}", e);
                    } else {
                        let _ = w.flush().await;
                        trace!("Peer message sent: {} bytes", data.len());
                    }
                }
                Err(_) => error!("Peer writer busy"),
            }
        });
    }
}

/// ClientSender
pub struct WriterClientSender {
    writer: Arc<tokio::sync::Mutex<WriteHalf<TcpStream>>>,
}

impl ClientSender for WriterClientSender {
    fn send(&self, msg: &TLVMessage) {
        let data = msg.encode();
        let writer = self.writer.clone();

        tokio::spawn(async move {
            match writer.try_lock() {
                Ok(mut w) => {
                    if let Err(e) = w.write_all(&data).await {
                        error!("Client send error: {}", e);
                    } else {
                        let _ = w.flush().await;
                        trace!("Client message sent: {} bytes", data.len());
                    }
                }
                Err(_) => error!("Client writer busy"),
            }
        });
    }
}

/// Handler für eingehende Verbindungen
pub async fn handle_connection(stream: TcpStream, handler: Arc<EventHandler>) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    let (mut reader, writer) = split(stream);
    let writer = Arc::new(tokio::sync::Mutex::new(writer));
    let client_id = rand::rng().next_u64() as ClientId;

    let client_sender = Arc::new(WriterClientSender { writer: writer.clone() });
    handler.clients.register(client_id, client_sender);
    info!("🔌 Client-ID {client_id} verbunden");

    for topic in &handler.public_topics {
        handler.handle_subscribe(topic, client_id);
        info!("📡 Auto-Subscribed: {}", String::from_utf8_lossy(topic));
    }

    let mut peer_id: Option<String> = None;

    // Erste Nachricht lesen
    if let Some(msg) = read_message(&mut reader).await {
        peer_id = process_initial_message(msg, &writer, &handler, client_id);
    } else {
        handler.clients.unregister(client_id);
        return Ok(());
    }

    // Haupt-Loop
    loop {
        match read_message(&mut reader).await {
            Some(msg) => {
                match msg.event_type {
                    EventType::Subscribe => {
                        if let Some(topic) = msg.get_field(FieldType::Key) {
                            handler.handle_subscribe(topic, client_id);
                        }
                    }
                    _ => {
                        handler.handle_event(&msg, peer_id.as_deref(), Some(client_id));
                    }
                }
            }
            None => {
                if let Some(id) = peer_id {
                    handler.peers.unregister(&id);
                } else {
                    handler.clients.unregister(client_id);
                }
                break;
            }
        }
    }

    Ok(())
}

/// Hilfsfunktion: liest eine vollständige Nachricht mit Header
async fn read_message(reader: &mut (impl AsyncReadExt + Unpin)) -> Option<TLVMessage> {
    let mut len_buf = [0u8; 4];
    if reader.read_exact(&mut len_buf).await.is_err() {
        return None;
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len < 5 || len > MAX_MESSAGE_SIZE {
        warn!("⚠️ Ungültige Länge: {}, Dump: {:?}", len, len_buf);
        return None;
    }

    let mut rest = vec![0u8; len - 4]; // jetzt korrekt: event_type + tlv payload
    if reader.read_exact(&mut rest).await.is_err() {
        return None;
    }

    let mut full = Vec::with_capacity(len);
    full.extend_from_slice(&len_buf);
    full.extend_from_slice(&rest);

    trace!("📥 Full message buffer: {:?}", full);

    TLVMessage::parse(Bytes::from(full)).ok()
}