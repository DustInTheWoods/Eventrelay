use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use crate::tlv::message::TLVMessage;

/// Trait für eine Verbindung zu einem Peer-Server
pub trait PeerSender: Send + Sync {
    fn send(&self, msg: &TLVMessage);
}

/// Registry für aktive Serververbindungen
pub struct PeerRegistry {
    pub(crate) peers: Mutex<HashMap<String, Arc<dyn PeerSender>>>, // Key = Peer-ID oder IP
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, id: String, sender: Arc<dyn PeerSender>) {
        info!("Registering peer with ID: {}", id);
        self.peers.lock().unwrap().insert(id, sender);
    }

    pub fn unregister(&self, id: &str) {
        info!("Unregistering peer with ID: {}", id);
        self.peers.lock().unwrap().remove(id);
    }

    /// Broadcastet die Nachricht an alle Peers außer dem angegebenen Ursprungsserver
    pub fn broadcast_to_peers(&self, msg: &TLVMessage, exclude_peer_id: Option<&str>) {
        let peers = self.peers.lock().unwrap();
        let peer_count = peers.len();
        debug!("Broadcasting message to {} peers (excluding: {:?})", peer_count, exclude_peer_id);

        let mut sent_count = 0;
        for (id, sender) in peers.iter() {
            if Some(id.as_str()) != exclude_peer_id {
                debug!("Sending message to peer: {}", id);
                sender.send(msg);
                sent_count += 1;
            }
        }

        debug!("Message broadcast completed. Sent to {}/{} peers", sent_count, peer_count);
    }

    /// Optional: explizit an bestimmten Peer senden
    pub fn send_to(&self, peer_id: &str, msg: &TLVMessage) {
        debug!("Attempting to send message to specific peer: {}", peer_id);
        if let Some(sender) = self.peers.lock().unwrap().get(peer_id) {
            debug!("Sending message to peer: {}", peer_id);
            sender.send(msg);
        } else {
            warn!("Failed to send message: Peer not found with ID: {}", peer_id);
        }
    }

    /// Gibt den Sender für einen bestimmten Peer zurück
    pub fn get(&self, peer_id: &str) -> Option<Arc<dyn PeerSender>> {
        debug!("Getting peer with ID: {}", peer_id);
        self.peers.lock().unwrap().get(peer_id).cloned()
    }

    /// Prüft, ob ein Peer mit der angegebenen ID existiert
    pub fn contains(&self, peer_id: &str) -> bool {
        debug!("Checking if peer exists with ID: {}", peer_id);
        self.peers.lock().unwrap().contains_key(peer_id)
    }
}
