use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use log::{debug, info, trace};
use crate::eventrelay::types::{ClientId, Topic, Subscriptions};

/// Speichert Subscriptions Thread-sicher
#[derive(Default)]
pub struct SubscriptionManager {
    inner: RwLock<Subscriptions>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Fügt eine Subscription hinzu
    pub fn subscribe(&self, topic: &[u8], client_id: ClientId) {
        let topic_str = String::from_utf8_lossy(topic);
        info!("Adding subscription: client {} to topic {}", client_id, topic_str);

        let topic_key = Arc::new(topic.to_vec());
        let mut subs = self.inner.write().unwrap();

        let is_new_topic = !subs.contains_key(&topic_key);
        if is_new_topic {
            debug!("Creating new topic entry for: {}", topic_str);
        }

        let client_set = subs.entry(topic_key)
            .or_insert_with(HashSet::new);

        let already_subscribed = !client_set.insert(client_id);
        if already_subscribed {
            debug!("Client {} was already subscribed to topic {}", client_id, topic_str);
        }

        trace!("Current subscription count for topic {}: {}", topic_str, client_set.len());
    }

    /// Entfernt eine Subscription
    pub fn unsubscribe(&self, topic: &[u8], client_id: ClientId) {
        let topic_str = String::from_utf8_lossy(topic);
        info!("Removing subscription: client {} from topic {}", client_id, topic_str);

        let topic_key = Arc::new(topic.to_vec());
        let mut subs = self.inner.write().unwrap();

        if let Some(set) = subs.get_mut(&topic_key) {
            let was_subscribed = set.remove(&client_id);
            if !was_subscribed {
                debug!("Client {} was not subscribed to topic {}", client_id, topic_str);
            }

            trace!("Remaining subscription count for topic {}: {}", topic_str, set.len());

            if set.is_empty() {
                debug!("Removing empty topic: {}", topic_str);
                subs.remove(&topic_key);
            }
        } else {
            debug!("Topic {} does not exist, nothing to unsubscribe", topic_str);
        }
    }

    /// Gibt alle Clients zurück, die ein Topic abonniert haben
    pub fn clients_for_topic(&self, topic: &[u8]) -> Vec<ClientId> {
        let topic_str = String::from_utf8_lossy(topic);
        debug!("Getting subscribed clients for topic: {}", topic_str);

        let topic_key = Arc::new(topic.to_vec());
        let subs = self.inner.read().unwrap();

        let result = subs.get(&topic_key)
            .map(|set| {
                let clients: Vec<ClientId> = set.iter().copied().collect();
                trace!("Found {} clients subscribed to topic {}", clients.len(), topic_str);
                clients
            })
            .unwrap_or_else(|| {
                debug!("No subscriptions found for topic {}", topic_str);
                Vec::new()
            });

        result
    }

    /// Gibt alle Subscriptions zurück (z. B. für Debug oder Broadcast-Checks)
    pub fn all(&self) -> HashMap<Topic, HashSet<ClientId>> {
        debug!("Retrieving all subscriptions");
        let subs = self.inner.read().unwrap().clone();
        info!("Total topics with subscriptions: {}", subs.len());
        subs
    }
}
