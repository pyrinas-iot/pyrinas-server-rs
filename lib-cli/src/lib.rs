pub mod ota;

use anyhow::anyhow;
use clap::{crate_version, Clap};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Websocket
use tungstenite::{client::AutoStream, http::Request, protocol::WebSocket};

/// Add a new OTA package to the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaAdd {
    /// UID to be directed to
    pub uid: String,
    /// JSON manifest file
    pub manifest: String,
    // Force the update
    #[clap(long, short)]
    pub force: bool,
}

/// Remove a OTA package from the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaRemove {
    /// UID to be directed to
    pub uid: String,
}

/// Config that can be installed locally
#[derive(Clap, Debug, Serialize, Deserialize)]
#[clap(version = crate_version!())]
pub struct Config {
    /// URL of the Pyrinas server to connect to.
    /// For example: pyrinas-admin.yourdomain.com
    pub url: String,
    /// Authentication key. This is the same key set in
    /// the Pyrinas config.toml
    pub authkey: String,
}

/// Configuration related commands
#[derive(Clap, Debug, Serialize, Deserialize)]
#[clap(version = crate_version!())]
pub struct ConfigCmd {
    #[clap(subcommand)]
    pub subcmd: ConfigSubCommand,
}

#[derive(Clap, Debug, Serialize, Deserialize)]
#[clap(version = crate_version!())]
pub enum ConfigSubCommand {
    Show(Show),
    Install(Config),
}

/// Show current configuration
#[derive(Clap, Debug, Serialize, Deserialize)]
#[clap(version = crate_version!())]
pub struct Show {}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAManifest {
    pub version: pyrinas_shared::OTAPackageVersion,
    pub file: String,
    pub force: bool,
}

pub fn get_socket(config: &Config) -> anyhow::Result<WebSocket<AutoStream>> {
    // String of full URL
    let full_uri = format!("wss://{}/socket", config.url);

    // Set up handshake request
    let req = Request::builder()
        .uri(full_uri)
        .header("ApiKey", config.authkey.clone())
        .body(())?;

    // Connect to TCP based WS socket
    // TODO: confirm URL is parsed correctly into tungstenite
    let (socket, _response) = match tungstenite::connect(req) {
        Ok(r) => r,
        Err(_e) => {
            return Err(anyhow!("Unable to connect to Websocket @ {}", config.url));
        }
    };

    // Return this guy
    Ok(socket)
}

/// Fetcht he configuration from the provided folder path
pub fn get_config() -> anyhow::Result<Config> {
    // Get config path
    let mut path = get_config_path()?;

    // Add file to path
    path.push("config.toml");

    // Read file to end
    let config = std::fs::read_to_string(path)?;

    // Deserialize
    let config: Config = toml::from_str(&config)?;
    Ok(config)
}

/// Set config
pub fn set_config(init: &Config) -> anyhow::Result<()> {
    // Get config path
    let mut path = get_config_path()?;

    // Create the config path
    std::fs::create_dir_all(&path)?;

    // Add file to path
    path.push("config.toml");

    // With init data create config.toml
    let config_string = toml::to_string(&init).unwrap();

    // Save config toml
    std::fs::write(path, config_string)?;

    Ok(())
}

fn get_config_path() -> anyhow::Result<PathBuf> {
    // Get the config file from standard location
    let mut config_path = match home::home_dir() {
        Some(path) => path,
        None => {
            return Err(anyhow!("Impossible to get your home dir!"));
        }
    };

    // Append config path to home directory
    config_path.push(".pyrinas");

    // Return it
    Ok(config_path)
}
