use tokio::task;
use log::{info, error, debug, warn, trace};
use env_logger::{self, Builder, Env};
use Eventrelay::eventrelay::config::ServerConfig;
use Eventrelay::eventrelay::init;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger with enhanced formatting and configuration
    // Default to info level if RUST_LOG is not set
    let env = Env::default().default_filter_or("debug");

    Builder::from_env(env)
        .format_timestamp_millis() // More precise timestamps with milliseconds
        .format_module_path(true)  // Include module path for better context
        .format_level(true)        // Include log level
        .format_target(false)      // Don't include target as it's redundant with module path
        .init();

    debug!("Logger initialized with enhanced formatting");


    info!("Eventrelay system starting up");
    trace!("System initialization process beginning");

    // Load configuration from config.toml
    debug!("Attempting to load configuration from config.toml");
    let config = match ServerConfig::from_toml_file("config.toml") {
        Ok(config) => {
            info!("Configuration loaded successfully");
            debug!("Configuration details: id={}", config.id);
            config
        },
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            warn!("System cannot start without valid configuration");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };

    let server = task::spawn(async move {
        info!("Starting server with ID: {}", config.id);
        debug!("Server initialization process beginning");
        if let Err(e) = init(config).await {
            error!("Error in server: {}", e);
            debug!("Server initialization failed, see error above");
        }
    });

    // Wait until the server is done (or crashes)
    let _ = tokio::join!(server);

    Ok(())
}
