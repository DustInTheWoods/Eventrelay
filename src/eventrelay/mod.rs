pub mod handler;
pub mod subscriptions;
pub mod client;
pub mod broadcaster;
pub mod net;
pub mod config;
mod types;

use std::sync::Arc;

use handler::EventHandler;
use client::ClientRegistry;
use subscriptions::SubscriptionManager;
use broadcaster::PeerRegistry;
use net::server::run_event_server;
use net::connector::connect_to_peers;
use config::ServerConfig;

/// Startet das Event-System mit Peer-Sync & Listener
pub async fn init(config: ServerConfig) -> std::io::Result<()> {
    let subscriptions = Arc::new(SubscriptionManager::new());
    let clients = Arc::new(ClientRegistry::new());
    let peers = Arc::new(PeerRegistry::new());

    // Convert public_channels from config to byte arrays
    let public_topics = config.public_channels
        .iter()
        .map(|channel| channel.as_bytes().to_vec())
        .collect();

    let handler = Arc::new(EventHandler::new(
        subscriptions,
        clients,
        peers.clone(),
        public_topics,
        true, // enable_subscriptions
    ));

    // Verbindung zu Peers aufbauen
    let peer_list = config
        .peers
        .iter()
        .map(|addr| (format!("peer-{}", addr), addr.clone()))
        .collect();
    connect_to_peers(peer_list, handler.clone(), config.id.clone()).await;

    // TCP Listener starten
    run_event_server(&config.addr, handler).await
}
