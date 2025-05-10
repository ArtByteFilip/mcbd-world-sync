use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs;
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub sync: SyncConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncConfig {
    pub devices: Vec<Device>,
    pub conflict_resolution: String,
    pub sync_interval: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathConfig {
    pub minecraft_worlds: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_str = fs::read_to_string("config.json")?;
        let config: Config = serde_json::from_str(&config_str)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_str = serde_json::to_string_pretty(self)?;
        fs::write("config.json", config_str)?;
        Ok(())
    }

    pub fn get_server_addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port).parse().unwrap()
    }
} 