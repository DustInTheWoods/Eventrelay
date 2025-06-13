use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use bytes::Bytes;
use log::{info, error, warn, debug};

use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::net::connection::Client;
use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};

/// Starts a background task that periodically checks for disconnected peers
/// and attempts to reconnect to them.
pub fn start_reconnection_task(
    peers: Vec<(String, String)>,
    handler: Arc<EventHandler>,
    self_id: String,
) {
    info!("Starting peer reconnection task");

    // Clone the data for the background task
    let peers_clone = peers.clone();
    let handler_clone = handler.clone();
    let self_id_clone = self_id.clone();

    // Spawn a background task
    tokio::spawn(async move {
        // Wait a bit before starting the reconnection loop to allow initial connections
        sleep(Duration::from_secs(5)).await;

        loop {
            // Check for disconnected peers and attempt to reconnect
            check_and_reconnect_peers(peers_clone.clone(), handler_clone.clone(), self_id_clone.clone()).await;

            // Wait before checking again
            sleep(Duration::from_secs(30)).await;
        }
    });
}

/// Checks for disconnected peers and attempts to reconnect to them
async fn check_and_reconnect_peers(
    peers: Vec<(String, String)>,
    handler: Arc<EventHandler>,
    self_id: String,
) {
    debug!("Checking for disconnected peers");

    for (peer_id, addr) in peers {
        let peer_id_bytes = Bytes::copy_from_slice(peer_id.as_bytes());

        // Check if the peer is already connected
        if handler.clients.contains_peer(&peer_id_bytes) {
            debug!("Peer {} is already connected", peer_id);
            continue;
        }

        // Determine which server should attempt reconnection
        // Use a deterministic approach based on server IDs
        if should_attempt_reconnection(&self_id, &peer_id) {
            debug!("This server is responsible for reconnecting to peer {}", peer_id);
            attempt_reconnect_to_peer(peer_id, addr, handler.clone(), self_id.clone()).await;
        } else {
            debug!("Waiting for peer {} to reconnect to us", peer_id);
        }
    }
}

/// Determines if this server should attempt reconnection to the peer
/// Uses a deterministic approach based on server IDs to avoid both servers
/// attempting reconnection simultaneously
fn should_attempt_reconnection(self_id: &str, peer_id: &str) -> bool {
    // Simple deterministic approach: the server with the "lower" ID is responsible
    // for reconnection attempts
    self_id < peer_id
}

/// Attempts to reconnect to a peer
async fn attempt_reconnect_to_peer(
    peer_id: String,
    addr: String,
    handler: Arc<EventHandler>,
    self_id: String,
) {
    info!("Attempting to reconnect to peer {} at {}", peer_id, addr);

    match TcpStream::connect(&addr).await {
        Ok(stream) => {
            // Set TCP_NODELAY to true to disable Nagle's algorithm
            if let Err(e) = stream.set_nodelay(true) {
                warn!("Failed to set TCP_NODELAY for peer {}: {}", peer_id, e);
            } else {
                debug!("TCP_NODELAY set successfully for peer {}", peer_id);
            }
            info!("Reconnected to peer {} at {}", peer_id, addr);

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
            error!("Reconnection to peer {} failed: {}", peer_id, e);
        }
    }
}
