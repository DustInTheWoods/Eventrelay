use std::collections::HashSet;

use log::{debug, info};
use bytes::Bytes;
use crate::eventrelay::types::{Topic, Subscriptions};
use dashmap::DashMap;

/// Stores subscriptions in a thread-safe manner
#[derive(Default)]
pub struct SubscriptionManager {
    inner: Subscriptions,
    public_topics: Vec<Bytes>,
}

impl SubscriptionManager {
    pub fn new(public_topics: Vec<Bytes>) -> Self {
        Self {
            inner: DashMap::new(),
            public_topics,
        }
    }

    /// Adds a subscription
    pub fn subscribe(&self, topic: &[u8], client_id: Bytes) {
        debug!("Adding subscription for client {:?} to topic", client_id);

        // Create the topic key
        let topic_key = topic.to_vec();

        // Use entry API for atomic operations
        self.inner.entry(topic_key).or_insert_with(HashSet::new).insert(client_id.clone());

        debug!("Added client {:?} to topic", client_id);
    }

    pub fn subscribe_public(&self, client_id: Bytes) {
        info!("Client {:?} subscribing to all public topics", client_id);
        
        for topic in self.public_topics.iter() {
            debug!("Subscribing client {:?} to public topic", client_id);
            self.subscribe(topic, client_id.clone());
        }
        
        info!("Client {:?} subscribed to all public topics", client_id);
    }

    /// Removes a subscription
    pub fn unsubscribe(&self, topic: &[u8], client_id: Bytes) {
        debug!("Removing subscription for client {:?} from topic", client_id);

        // Create the topic key
        let topic_key = topic.to_vec();

        // Use entry API for atomic operations
        if let Some(mut entry) = self.inner.get_mut(&topic_key) {
            entry.remove(&client_id);
            debug!("Removed client {:?} from topic", client_id);

            // If the set is empty, remove the topic
            if entry.is_empty() {
                drop(entry); // Release the reference before removing
                self.inner.remove(&topic_key);
                debug!("Removed empty topic");
            }
        }

        debug!("Topic not found for unsubscribe");
    }

    pub fn unsubscribe_public(&self, client_id: Bytes) {
        info!("Client {:?} unsubscribing from all public topics", client_id);
        
        for topic in self.public_topics.iter() {
            debug!("Unsubscribing client {:?} from public topic", client_id);
            self.unsubscribe(topic, client_id.clone());
        }
        
        info!("Client {:?} unsubscribed from all public topics", client_id);
    }

    /// Returns all clients that have subscribed to a topic
    pub fn clients_for_topic(&self, topic: &[u8]) -> Vec<Bytes> {
        debug!("Getting clients for topic");

        // Create the topic key
        let topic_key = topic.to_vec();

        // Direct lookup using the key
        if let Some(entry) = self.inner.get(&topic_key) {
            // Pre-allocate the vector with the exact size needed
            let mut clients = Vec::with_capacity(entry.len());
            clients.extend(entry.iter().cloned());
            debug!("Found {} clients for topic", clients.len());
            clients
        } else {
            debug!("No clients found for topic");
            Vec::new()
        }
    }

    /// Gibt alle Subscriptions zurück (z. B. für Debug oder Broadcast-Checks)
    pub fn all(&self) -> DashMap<Topic, HashSet<Bytes>> {
        debug!("Retrieving all subscriptions");
        self.inner.clone()
    }

    /// Unsubscribes a client from all topics
    pub fn unsubscribe_all(&self, client_id: Bytes) {
        info!("Client {:?} unsubscribing from all topics", client_id);

        // First unsubscribe from all public topics
        self.unsubscribe_public(client_id.clone());

        // Then unsubscribe from all other topics
        // We need to collect the topics first to avoid modifying the map while iterating
        let topics_to_unsubscribe: Vec<Topic> = self.inner
            .iter()
            .filter_map(|entry| {
                if entry.value().contains(&client_id) {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        // Now unsubscribe from each topic
        for topic in topics_to_unsubscribe {
            debug!("Unsubscribing client {:?} from topic", client_id);
            self.unsubscribe(&topic, client_id.clone());
        }

        info!("Client {:?} unsubscribed from all topics", client_id);
    }
}
