use config::{Config, ConfigError, File};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Mqtt {
  pub host: String,
  pub port: String,
  pub ca_cert: String,
  pub server_cert: String,
  pub private_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Influx {
  pub database: String,
  pub host: String,
  pub password: String,
  pub port: String,
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
pub struct Sled {
  pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
  pub influx: Influx,
  pub mqtt: Mqtt,
  pub sock: Sock,
  pub s3: S3,
  pub sled: Sled,
}

impl Settings {
  pub fn new() -> Result<Self, ConfigError> {
    let mut s = Config::new();

    // Get the configuration file
    s.merge(File::with_name("config"))?;

    // You can deserialize (and thus freeze) the entire configuration as
    s.try_into()
  }
}
