use crate::eventrelay::subscriptions::SubscriptionManager;
use crate::eventrelay::broadcaster::ConnectionRegistry;

use std::sync::Arc;
use log::{debug, error, info, warn};
use bytes::Bytes;
use crate::tlv::message::TLVMessage;

/// Manages event distribution in the cluster

pub struct EventHandler {
    pub subscriptions: Arc<SubscriptionManager>,
    pub clients: Arc<ConnectionRegistry>,
    pub public_topics: Vec<Vec<u8>>, 
}

impl EventHandler {
    pub fn new(
        subscriptions: Arc<SubscriptionManager>,
        clients: Arc<ConnectionRegistry>,
        public_topics: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            subscriptions,
            clients,
            public_topics,
        }
    }

    /// Called when an event is received
    pub fn handle_event(&self, topic: &[u8], msg: &TLVMessage, source_peer: Option<Bytes>, source_client: Option<Bytes>) {
        // Reduce logging in this performance-critical path
        if log::log_enabled!(log::Level::Info) {
            info!("Handling incoming event from peer: {:?}, client: {:?}", source_peer, source_client);
        }

        if let Some(ref peer_id) = source_peer {
            debug!("Received message from server: {:?}", peer_id);
        }

        // Local clients
        // Always send it to subscribed clients first
        let client_ids = self.subscriptions.clients_for_topic(topic);

        debug!("Sending to {} subscribed clients (excluding source: {:?})", client_ids.len(), source_client);

        // Send to each subscribed client
        for client_id in &client_ids {
            if Some(client_id) != source_client.as_ref() {
                if let Some(client) = self.clients.get_client(client_id) {
                    if let Err(e) = client.send_tlv(msg) {
                        error!("Failed to send message to client {:?}: {}", client_id, e);
                    }
                }
            }
        }

        // Forward to other servers, but only if the message doesn't come from a server
        if source_peer.is_none() {
            debug!("Forwarding event to other servers");
            self.clients.broadcast_to_peers(msg, None);
        }
        debug!("Not forwarding server-originated event to other servers");

        info!("Event handling completed");
    }

    pub fn handle_subscribe(&self, topic: &[u8], client_id: Bytes) {
        info!("Client {:?} subscribing to topic", client_id);
        self.subscriptions.subscribe(topic, client_id);
    }

    pub fn handle_unsubscribe(&self, topic: &[u8], client_id: Bytes) {
        info!("Client {:?} unsubscribing from topic", client_id);
        self.subscriptions.unsubscribe(topic, client_id);
    }

    pub fn handle_subscribe_public(&self, client_id: Bytes) {
        info!("Client {:?} subscribing to public topic", client_id);
        self.subscriptions.subscribe_public(client_id);
    }

    pub fn handle_unsubscribe_public(&self, client_id: Bytes) {
        info!("Client {:?} unsubscribing from public topic", client_id);
        self.subscriptions.unsubscribe_public(client_id);
    }

    /// Handles cleanup when a client disconnects
    /// This unsubscribes the client from all topics and unregisters it from the connection registry
    pub fn handle_client_disconnect(&self, client_id: Bytes) {
        info!("Handling disconnect for client {:?}", client_id);

        // Unsubscribe from all topics
        self.subscriptions.unsubscribe_all(client_id.clone());

        // Unregister from the connection registry
        if self.clients.contains_client(&client_id) {
            self.clients.unregister_client(&client_id);
            info!("Client {:?} unregistered from connection registry", client_id);
        } else if self.clients.contains_peer(&client_id) {
            self.clients.unregister_peer(&client_id);
            info!("Peer {:?} unregistered from connection registry", client_id);
        }

        info!("Client {:?} cleanup completed", client_id);
    }
}
