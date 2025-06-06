use std::io::{stdin, stdout, Write};
use std::time::Duration;
use tokio::task;
use log::{info, error};
use env_logger;
use Eventrelay::eventrelay::config::ServerConfig;
use Eventrelay::eventrelay::init;
use Eventrelay::error::ReplicashError;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger with timestamp and log level
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .format_level(true)
        .init();


    info!("Eventrelay system starting up");

    // Load configuration from config.toml
    let config = match ServerConfig::from_toml_file("config.toml") {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        },
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    let server = task::spawn(async move {
        info!("🚀 Starting server {}", config.id);
        if let Err(e) = init(config).await {
            error!("❌ Error in server: {e}");
        }
    });

    // Wait until the server is done (or crashes)
    let _ = tokio::join!(server);

    Ok(())
}