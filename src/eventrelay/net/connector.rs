use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use bytes::Bytes;
use log::{info, error, warn, debug};
use tokio::time::{sleep, Duration};

use crate::eventrelay::broadcaster::{PeerRegistry, PeerSender};
use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::net::connection::handle_connection;
use crate::tlv::types;
use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};

/// Dummy PeerSender – schickt Events über TCP
pub struct TcpPeerSender {
    stream: Arc<tokio::sync::Mutex<TcpStream>>,
}

impl TcpPeerSender {
    pub fn new(mut stream: TcpStream) -> Arc<dyn PeerSender> {
        // Set TCP_NODELAY to true to disable Nagle's algorithm
        if let Err(e) = stream.set_nodelay(true) {
            error!("❌ Failed to set TCP_NODELAY for TcpPeerSender: {e}");
        }
        Arc::new(Self {
            stream: Arc::new(tokio::sync::Mutex::new(stream)),
        })
    }
}

impl PeerSender for TcpPeerSender {
    fn send(&self, msg: &TLVMessage) {
        // Encode the message before spawning the task
        let data = msg.encode();

        // Clone what we need for the async task
        let data_clone = data.clone();
        let stream_clone = self.stream.clone();

        // Log message details
        debug!("TcpPeerSender preparing to send message: event_type={:?}, size={} bytes", 
               msg.event_type, data.len());

        // Use tokio::spawn to handle the async write operation
        let handle = tokio::spawn(async move {
            // Get a lock on the stream
            debug!("TcpPeerSender acquiring lock on stream");
            let mut stream = match stream_clone.try_lock() {
                Ok(stream) => stream,
                Err(_) => {
                    error!("TcpPeerSender failed to acquire lock on stream - stream is busy");
                    return;
                }
            };

            debug!("TcpPeerSender sending message: {} bytes", data_clone.len());

            // Write the data and handle errors
            match stream.write_all(&data_clone).await {
                Ok(_) => {
                    // Try to flush the stream to ensure data is sent
                    match stream.flush().await {
                        Ok(_) => debug!("TcpPeerSender message sent and flushed successfully"),
                        Err(e) => error!("TcpPeerSender failed to flush after sending message: {}", e)
                    }
                },
                Err(e) => {
                    error!("TcpPeerSender failed to send message: {}", e);
                }
            }
        });

        // Log that we've spawned the task
        debug!("TcpPeerSender spawned async task to send message");
    }
}

/// Baut Verbindungen zu allen bekannten Peers auf
pub async fn connect_to_peers(
    peers: Vec<(String, String)>,
    handler: Arc<EventHandler>, // Statt nur registry, direkt der Handler
    self_id: String,
) {
    for (peer_id, addr) in peers {
        match TcpStream::connect(&addr).await {
            Ok(mut stream) => {
                // Set TCP_NODELAY to true to disable Nagle's algorithm
                if let Err(e) = stream.set_nodelay(true) {
                    error!("❌ Failed to set TCP_NODELAY for peer {peer_id}: {e}");
                }
                info!("🔗 Connected to peer {peer_id} @ {addr}");

                // 🟢 SyncInit senden
                let mut init_msg = TLVMessage::new(EventType::SyncInit);
                init_msg.insert_field(FieldType::Key, Bytes::copy_from_slice(self_id.as_bytes()));
                if let Err(e) = stream.write_all(&init_msg.encode()).await {
                    error!("❌ Error sending SyncInit to {peer_id}: {e}");
                    continue;
                }

                // ⬇️ Starte den Handler in einem Task (zum Lesen + Weiterverarbeiten)
                let handler_clone = handler.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, handler_clone).await {
                        error!("❌ Peer handler for {peer_id} failed: {e}");
                    }
                });
            }
            Err(e) => {
                error!("❌ Connection to peer {peer_id} failed: {e}");
            }
        }
    }
}
