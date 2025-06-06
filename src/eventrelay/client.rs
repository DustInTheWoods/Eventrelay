use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::{debug, info};
use crate::eventrelay::types::ClientId;
use crate::tlv::message::TLVMessage;

/// Trait für einen beliebigen Event-Sender an Clients
pub trait ClientSender: Send + Sync {
    fn send(&self, msg: &TLVMessage);
}

/// Verwaltung aller verbundenen Clients
pub struct ClientRegistry {
    clients: Mutex<HashMap<ClientId, Arc<dyn ClientSender>>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, id: ClientId, sender: Arc<dyn ClientSender>) {
        info!("Registering client with ID: {:?}", id);
        self.clients.lock().unwrap().insert(id, sender);
    }

    pub fn unregister(&self, id: ClientId) {
        info!("Unregistering client with ID: {:?}", id);
        self.clients.lock().unwrap().remove(&id);
    }

    pub fn send_to(&self, client_ids: &[ClientId], msg: &TLVMessage) {
        debug!("Sending message to specific clients: {:?}", client_ids);
        let clients = self.clients.lock().unwrap();
        let mut sent_count = 0;
        for id in client_ids {
            if let Some(sender) = clients.get(id) {
                debug!("Sending message to client: {:?}", id);
                sender.send(msg);
                sent_count += 1;
            } else {
                debug!("Client not found: {:?}", id);
            }
        }
        debug!("Message sent to {}/{} clients", sent_count, client_ids.len());
    }

    pub fn send_to_except(&self, client_ids: &[ClientId], msg: &TLVMessage, exclude_id: Option<ClientId>) {
        debug!("Sending message to specific clients: {:?} (excluding: {:?})", client_ids, exclude_id);
        let clients = self.clients.lock().unwrap();
        let mut sent_count = 0;
        for id in client_ids {
            if exclude_id.map_or(true, |exclude| *id != exclude) {
                if let Some(sender) = clients.get(id) {
                    debug!("Sending message to client: {:?}", id);
                    sender.send(msg);
                    sent_count += 1;
                } else {
                    debug!("Client not found: {:?}", id);
                }
            } else {
                debug!("Skipping excluded client: {:?}", id);
            }
        }
        debug!("Message sent to {}/{} clients (excluding excluded client)", sent_count, client_ids.len());
    }

    pub fn broadcast(&self, msg: &TLVMessage) {
        let clients = self.clients.lock().unwrap();
        let client_count = clients.len();
        debug!("Broadcasting message to all {} clients", client_count);

        if client_count > 0 {
            let client_ids: Vec<ClientId> = clients.keys().cloned().collect();
            debug!("Client IDs receiving broadcast: {:?}", client_ids);

            for (id, sender) in clients.iter() {
                debug!("Sending message to client: {:?}", id);
                sender.send(msg);
            }
        }

        debug!("Broadcast completed to {} clients", client_count);
    }

    pub fn broadcast_except(&self, msg: &TLVMessage, exclude_id: Option<ClientId>) {
        let clients = self.clients.lock().unwrap();
        let client_count = clients.len();
        debug!("Broadcasting message to all clients (excluding: {:?})", exclude_id);

        let mut sent_count = 0;
        if client_count > 0 {
            let included_clients: Vec<ClientId> = clients.keys()
                .filter(|id| exclude_id.map_or(true, |exclude| **id != exclude))
                .cloned()
                .collect();

            debug!("Client IDs receiving broadcast: {:?}", included_clients);

            for (id, sender) in clients.iter() {
                if exclude_id.map_or(true, |exclude| *id != exclude) {
                    debug!("Sending message to client: {:?}", id);
                    sender.send(msg);
                    sent_count += 1;
                } else {
                    debug!("Skipping excluded client: {:?}", id);
                }
            }
        }

        debug!("Broadcast completed to {}/{} clients (excluding excluded client)", sent_count, client_count);
    }
}
