use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use toml;

#[derive(Debug, Deserialize, Clone)]
pub struct Mqtt {
    pub name: String,
    pub topics: Vec<String>,
    pub rumqtt: librumqttd::Config,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Influx {
    pub database: String,
    pub host: String,
    pub password: String,
    pub port: u16,
    pub user: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Sock {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ota {
    pub db_path: String,
    pub http_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PyrinasSettings {
    pub server_url: String,
    pub influx: Influx,
    pub mqtt: Mqtt,
    pub sock: Sock,
    pub ota: Ota,
}

impl PyrinasSettings {
    pub fn new(config: String) -> Result<Self> {
        // Get the path
        let path = Path::new(&config);

        // Get it as a string first
        let config = fs::read_to_string(path)?;

        // Get the actual config
        match toml::from_str(&config) {
            Ok(settings) => Ok(settings),
            Err(e) => Err(anyhow!("Unable to deserialize TOML: {}", e)),
        }
    }
}
