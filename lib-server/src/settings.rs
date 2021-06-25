use crate::Error;
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

/// Struct for Admin interface
#[derive(Debug, Deserialize, Clone)]
pub struct Admin {
    /// Port for the Websocket admin interface
    pub port: u16,
    /// Api key for Admin interface
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ota {
    pub url: String,
    pub db_path: String,
    pub http_port: u16,
    pub image_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PyrinasSettings {
    pub influx: Option<Influx>,
    pub mqtt: Mqtt,
    pub admin: Option<Admin>,
    pub ota: Ota,
}

impl PyrinasSettings {
    pub fn new(config: String) -> Result<Self, Error> {
        // Get the path
        let path = Path::new(&config);

        // Get it as a string first
        let config = fs::read_to_string(path)?;

        // Get the actual config
        match toml::from_str(&config) {
            Ok(settings) => Ok(settings),
            Err(e) => Err(Error::CustomError(format!(
                "Unable to deserialize TOML: {}",
                e
            ))),
        }
    }
}
