use tokio::net::TcpListener;
use std::sync::Arc;
use log::{info, warn};

use crate::eventrelay::handler::EventHandler;
use crate::eventrelay::net::connection::handle_connection;

/// Startet den TCP-Listener (als Platzhalter für echten QUIC)
pub async fn run_event_server(addr: &str, handler: Arc<EventHandler>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("📡 Event server running on {}", addr);

    loop {
        let (mut stream, addr) = listener.accept().await?;
        // Set TCP_NODELAY to true to disable Nagle's algorithm
        if let Err(e) = stream.set_nodelay(true) {
            warn!("⚠️ Failed to set TCP_NODELAY for connection from {}: {}", addr, e);
        }
        info!("🔌 New connection from {}", addr);

        let handler = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, handler).await {
                warn!("⚠️ Connection error for {}: {:?}", addr, e);
            }
        });
    }
}
