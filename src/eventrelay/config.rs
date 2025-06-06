use std::net::SocketAddr;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::error::{ReplicashError, ErrorCode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: String,
    pub addr: String,
    pub peers: Vec<String>,
    pub public_channels: Vec<String>,
}

// Keep for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub address: SocketAddr,
}

impl ServerConfig {
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, ReplicashError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ReplicashError::new(ErrorCode::ConfigInvalid, format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| ReplicashError::new(ErrorCode::ConfigInvalid, format!("Failed to parse TOML: {}", e)))
    }

    /// Save configuration to a TOML file
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ReplicashError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ReplicashError::new(ErrorCode::ConfigInvalid, format!("Failed to serialize to TOML: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| ReplicashError::new(ErrorCode::ConfigInvalid, format!("Failed to write config file: {}", e)))
    }
}
