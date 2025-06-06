use crate::tlv::types;
use crate::eventrelay::subscriptions::SubscriptionManager;
use crate::eventrelay::client::ClientRegistry;
use crate::eventrelay::broadcaster::PeerRegistry;
use crate::eventrelay::types::*;

use std::sync::Arc;
use log::{debug, info, warn};
use crate::tlv::message::TLVMessage;
use crate::tlv::types::{EventType, FieldType};

/// Verwaltet Event-Verteilung im Cluster
pub struct EventHandler {
    pub subscriptions: Arc<SubscriptionManager>,
    pub clients: Arc<ClientRegistry>,
    pub peers: Arc<PeerRegistry>,
    pub public_topics: Vec<Vec<u8>>, // z. B. "chat/global"
    pub enable_subscriptions: bool,
}

impl EventHandler {
    pub fn new(
        subscriptions: Arc<SubscriptionManager>,
        clients: Arc<ClientRegistry>,
        peers: Arc<PeerRegistry>,
        public_topics: Vec<Vec<u8>>,
        enable_subscriptions: bool,
    ) -> Self {
        Self {
            subscriptions,
            clients,
            peers,
            public_topics,
            enable_subscriptions,
        }
    }

    /// Wird aufgerufen, wenn ein Event eingeht
    pub fn handle_event(&self, msg: &TLVMessage, source_peer: Option<&str>, source_client: Option<ClientId>) {
        info!("Handling incoming event from peer: {:?}, client: {:?}", source_peer, source_client);

        if let Some(peer_id) = source_peer {
            debug!("📥 Received message from server: {}", peer_id);
        }

        let topic = match msg.get_field(FieldType::Key) {
            Some(val) => {
                let topic_ref = val.as_ref();
                debug!("Event topic: {:?}", String::from_utf8_lossy(topic_ref));
                topic_ref
            },
            None => {
                warn!("Event ohne Topic-Key erhalten");
                return;
            }
        };

        let is_public = self.public_topics.iter().any(|t| t == topic);
        debug!("Topic is public: {}", is_public);

        // Lokale Clients
        if self.enable_subscriptions {
            // Immer zuerst an abonnierte Clients senden
            let client_ids = self.subscriptions.clients_for_topic(topic);
            debug!("Sending to {} subscribed clients (excluding source: {:?})", client_ids.len(), source_client);
            self.clients.send_to_except(&client_ids, msg, source_client);
        } else {
            debug!("Subscriptions disabled, broadcasting to all local clients (excluding source: {:?})", source_client);
            self.clients.broadcast_except(msg, source_client);
        }

        // Weiterleiten an andere Server, aber nur wenn die Nachricht nicht von einem Server kommt
        if source_peer.is_none() {
            debug!("Forwarding event to other servers");
            self.peers.broadcast_to_peers(msg, None);
        } else {
            debug!("Not forwarding server-originated event to other servers");
        }

        info!("Event handling completed");
    }

    /// Wird aufgerufen, wenn ein Client ein Topic abonniert
    pub fn handle_subscribe(&self, topic: &[u8], client_id: ClientId) {
        let topic_str = String::from_utf8_lossy(topic);
        info!("Client {} subscribing to topic: {}", client_id, topic_str);

        if self.enable_subscriptions {
            debug!("Subscriptions enabled, adding subscription");
            self.subscriptions.subscribe(topic, client_id);
        } else {
            debug!("Subscriptions disabled, ignoring subscription request");
        }
    }

    /// Optional: explizite Unsubscription
    pub fn handle_unsubscribe(&self, topic: &[u8], client_id: ClientId) {
        let topic_str = String::from_utf8_lossy(topic);
        info!("Client {} unsubscribing from topic: {}", client_id, topic_str);

        if self.enable_subscriptions {
            debug!("Subscriptions enabled, removing subscription");
            self.subscriptions.unsubscribe(topic, client_id);
        } else {
            debug!("Subscriptions disabled, ignoring unsubscription request");
        }
    }
}
