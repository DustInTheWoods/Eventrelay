pub mod handler;
pub mod subscriptions;
pub mod broadcaster;
pub mod net;
pub mod config;
mod types;


use std::sync::Arc;
use bytes::Bytes;

use handler::EventHandler;
use subscriptions::SubscriptionManager;
use net::server::run_event_server;
use net::connector::connect_to_peers;
use net::reconnector::start_reconnection_task;
use config::ServerConfig;
use crate::eventrelay::broadcaster::ConnectionRegistry;

/// Startet das Event-System mit Peer-Sync & Listener
pub async fn init(config: ServerConfig) -> std::io::Result<()> {
    // Convert public_channels from config to Bytes
    let public_channels_bytes = config.public_channels
        .iter()
        .map(|channel| Bytes::from(channel.clone()))
        .collect();
    let subscriptions = Arc::new(SubscriptionManager::new(public_channels_bytes));
    let clients = Arc::new(ConnectionRegistry::new());

    // Convert public_channels from config to byte arrays
    let public_topics = config.public_channels
        .iter()
        .map(|channel| channel.as_bytes().to_vec())
        .collect();

    let handler = Arc::new(EventHandler::new(
        subscriptions,
        clients,
        public_topics,
    ));

    // Verbindung zu Peers aufbauen
    let peer_list: Vec<(String, String)> = config
        .peers
        .iter()
        .map(|addr| (format!("peer-{}", addr), addr.clone()))
        .collect();

    // Initial connection to peers
    connect_to_peers(peer_list.clone(), handler.clone(), config.id.clone()).await;

    // Start the reconnection task for handling disconnected peers
    start_reconnection_task(peer_list, handler.clone(), config.id.clone());

    // TCP Listener starten
    run_event_server(&config.addr, handler).await
}
