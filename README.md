# Eventrelay

A relay system for events across a network of peers.

## Configuration

Eventrelay now supports TOML configuration files. This makes it easier to configure the server with a human-readable format.

### Sample Configuration

Here's a sample TOML configuration file:

```toml
# ================================
# 📡 EventRelay Server Configuration
# ================================

# 🎯 Eindeutige Kennung dieses Servers
id = "server1"

# 🔌 Adresse für eingehende Verbindungen
bind_addr = "127.0.0.1:8080"

# 🌐 Bekannte Peer-Server
peers = [
    "127.0.0.1:8081",
    "127.0.0.1:8082"
]

# 📣 Öffentliche Topics zur Verteilung
public_channels = [
    "announcements",
    "general",
    "support"
]
```

### Loading Configuration

You can load a configuration from a TOML file using the `ServerConfig::from_toml_file` method:

```rust
use std::path::Path;
use Eventrelay::eventrelay::config::ServerConfig;

// Load configuration from TOML file
let config_path = Path::new("config.toml");
let config = ServerConfig::from_toml_file(config_path)?;

// Use the configuration
println!("Server ID: {}", config.id);
```

### Saving Configuration

You can save a configuration to a TOML file using the `ServerConfig::to_toml_file` method:

```rust
// Save configuration to TOML file
let config_path = Path::new("config.toml");
config.to_toml_file(config_path)?;
```

### Example

See the `examples/config_example.rs` file for a complete example of loading, modifying, and saving a configuration.

Run the example with:

```
cargo run --bin config_example
```
