use std::sync::Arc;
use tokio::net::TcpStream;
use bytes::Bytes;
use log::{info, error, warn, debug};

use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::net::connection::{Client};
use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};


/// Establishes connections to all known peers
pub async fn connect_to_peers(
    peers: Vec<(String, String)>,
    handler: Arc<EventHandler>, // Using the handler directly instead of just the registry
    self_id: String,
) {
    info!("Connecting to {} known peers", peers.len());

    for (peer_id, addr) in peers {
        debug!("Attempting to connect to peer {} at {}", peer_id, addr);
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                // Set TCP_NODELAY to true to disable Nagle's algorithm
                if let Err(e) = stream.set_nodelay(true) {
                    warn!("Failed to set TCP_NODELAY for peer {}: {}", peer_id, e);
                } else {
                    debug!("TCP_NODELAY set successfully for peer {}", peer_id);
                }
                info!("Connected to peer {} at {}", peer_id, addr);

                // Create a new client for the connection
                let client = Client::new(stream, handler.clone());

                // Register the client
                handler.clients.register_client(client.id(), client.clone());

                // Create a SyncInit message and send it
                let mut tlv_message = TLVMessage::new(EventType::SyncInit);
                tlv_message.insert_field(FieldType::Key, Bytes::copy_from_slice(self_id.as_bytes()));

                match client.send_tlv(&tlv_message) {
                    Ok(_) => info!("SyncInit sent to peer {}", peer_id),
                    Err(e) => error!("Error sending SyncInit to {}: {}", peer_id, e)
                }
            }
            Err(e) => {
                error!("Connection to peer {} failed: {}", peer_id, e);
            }
        }
    }
}
