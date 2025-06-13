use dashmap::DashMap;
use std::sync::Arc;
use bytes::Bytes;
use log::{debug, error, info, warn};
use crate::eventrelay::net::connection::Client;
use crate::tlv::message::TLVMessage;

/// Registry for active peer connections
pub struct ConnectionRegistry {
    peers: DashMap<Bytes, Arc<Client>>,
    clients: DashMap<Bytes, Arc<Client>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            clients: DashMap::new(),
        }
    }

    pub fn register_peer(&self, id: Bytes, client: Arc<Client>) {
        info!("Registering peer with ID: {:?}", id);
        self.peers.insert(id, client);
    }

    pub fn register_client(&self, id: Bytes, client: Arc<Client>) {
        info!("Registering client with id: {:?}", id);
        self.clients.insert(id, client);
    }

    pub fn unregister_peer(&self, id: &Bytes) {
        info!("Unregistering peer with ID: {:?}", id);
        self.peers.remove(id);
    }

    pub fn unregister_client(&self, id: &Bytes) {
        info!("Unregistering client with id: {:?}", id);
        self.clients.remove(id);
    }

    pub fn broadcast_to_peers(&self, msg: &TLVMessage, exclude_peer_id: Option<&Bytes>) {
        debug!("Broadcasting message to {} peers", self.peers.len());

        // For all peers, use regular iteration with conditional logging
        for peer in self.peers.iter() {
            if Some(peer.key()) != exclude_peer_id {
                if let Err(e) = peer.value().send_tlv(msg) {
                    error!("Failed to send message: {}, to peer: {:?}", e, peer.key());
                }
            }
        }

        debug!("Broadcast to peers completed");
    }

    pub fn broadcast_to_clients(&self, msg: &TLVMessage, exclude_client_id: Option<&Bytes>) {
        debug!("Broadcasting message to {} clients", self.clients.len());

        // For all clients, use regular iteration with conditional logging
        for client in self.clients.iter() {
            if Some(client.key()) != exclude_client_id {
                if let Err(e) = client.value().send_tlv(msg) {
                    error!("Failed to send message: {}, to client: {:?}", e, client.key());
                }           
            }
        }

        debug!("Broadcast to clients completed");
    }

    pub fn get_peer(&self, peer_id: &Bytes) -> Option<Arc<Client>> {
        let result = self.peers.get(peer_id).map(|entry| entry.value().clone());
        if result.is_some() {
            debug!("Found peer with ID: {:?}", peer_id);
        } else {
            debug!("Peer with ID: {:?} not found", peer_id);
        }
        result
    }

    pub fn get_client(&self, client_id: &Bytes) -> Option<Arc<Client>> {
        let result = self.clients.get(client_id).map(|entry| entry.value().clone());
        if result.is_some() {
            debug!("Found client with ID: {:?}", client_id);
        } else {
            debug!("Client with ID: {:?} not found", client_id);
        }
        result
    }

    pub fn contains_peer(&self, peer_id: &Bytes) -> bool {
        let contains = self.peers.contains_key(peer_id);
        debug!("Checking if peer with ID: {:?} exists: {}", peer_id, contains);
        
        contains
    }

    pub fn contains_client(&self, client_id: &Bytes) -> bool {
        let contains = self.clients.contains_key(client_id);
        debug!("Checking if client with ID: {:?} exists: {}", client_id, contains);
        contains
    }
}
