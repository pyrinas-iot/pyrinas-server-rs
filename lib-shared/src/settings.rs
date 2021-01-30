use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use toml;

#[derive(Debug, Deserialize, Clone)]
pub struct Mqtt {
  pub id: String,
  pub host: String,
  pub port: u16,
  pub ca_cert: String,
  pub server_cert: String,
  pub private_key: String,
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
pub struct S3 {
  pub access_key: String,
  pub bucket: String,
  pub region: String,
  pub secret_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtaDb {
  pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PyrinasSettings {
  pub influx: Influx,
  pub mqtt: Mqtt,
  pub sock: Sock,
  pub s3: S3,
  pub ota_db: OtaDb,
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
