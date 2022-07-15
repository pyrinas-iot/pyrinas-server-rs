pub mod certs;
pub mod config;
pub mod device;
pub mod git;
pub mod ota;

use clap::Parser;
use pyrinas_shared::{ota::OTAPackageVersion, OtaLink};
use serde::{Deserialize, Serialize};
use std::{net::TcpStream, num};

// Error handling
use thiserror::Error;

// Websocket
use tungstenite::{client::IntoClientRequest, http, protocol::WebSocket, stream::MaybeTlsStream};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{source}")]
    Error {
        #[from]
        source: config::Error,
    },

    #[error("http error: {source}")]
    HttpError {
        #[from]
        source: http::Error,
    },

    #[error("websocket handshake error {source}")]
    WebsocketError {
        #[from]
        source: tungstenite::Error,
    },

    #[error("semver error: {source}")]
    SemVerError {
        #[from]
        source: semver::Error,
    },

    #[error("parse error: {source}")]
    ParseError {
        #[from]
        source: num::ParseIntError,
    },

    #[error("err: {0}")]
    CustomError(String),

    #[error("ota error: {source}")]
    OtaError {
        #[from]
        source: ota::Error,
    },

    #[error("{source}")]
    CertsError {
        #[from]
        source: certs::Error,
    },
}

/// Various commands related to the OTA process
#[derive(Parser, Debug)]
#[clap(version)]
pub struct OtaCmd {
    #[clap(subcommand)]
    pub subcmd: OtaSubCommand,
}

/// Commands related to certs
#[derive(Parser, Debug)]
#[clap(version)]
pub struct CertCmd {
    #[clap(subcommand)]
    pub subcmd: CertSubcommand,
}

#[derive(Parser, Debug)]
#[clap(version)]
pub enum CertSubcommand {
    /// Generate CA cert
    Ca,
    /// Generate server cert
    Server,
    /// Generate device cert
    Device(CertDevice),
}

/// Remove a OTA package from the sever
#[derive(Parser, Debug)]
#[clap(version)]
pub struct CertDevice {
    /// ID of the device (usually IMEI). Obtains from device if not provided.
    id: Option<String>,
    /// Automatic provision
    #[clap(long, short)]
    provision: bool,
    /// Provision to device using DER
    #[clap(long, short)]
    der: bool,
    /// Serial port
    #[clap(long, default_value = certs::DEFAULT_MAC_PORT )]
    port: String,
    /// Security tag for provisioning
    #[clap(long, short)]
    tag: Option<u32>,
}

#[derive(Parser, Debug)]
#[clap(version)]
pub enum OtaSubCommand {
    /// Add OTA package
    Add(OtaAdd),
    /// Associate command
    Link(OtaLink),
    /// Unlink from Device/Group
    Unlink(OtaLink),
    /// Remove OTA package
    Remove(OtaRemove),
    /// List groups
    ListGroups,
    /// List images
    ListImages,
}

/// Add a OTA package from the sever
#[derive(Parser, Debug)]
#[clap(version)]
pub struct OtaAdd {
    /// Force updating in dirty repository
    #[clap(long, short)]
    pub force: bool,
    /// Option to autmoatically associate with device.
    /// Device group also set to device id.
    #[clap(long, short)]
    pub device_id: Option<String>,
    /// Optional path to binary file
    #[clap(long, short)]
    pub bin: Option<String>,
}

/// Remove a OTA package from the sever
#[derive(Parser, Debug)]
#[clap(version)]
pub struct OtaRemove {
    /// Image id to be directed to
    pub image_id: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CertConfig {
    /// Domain certs are being generated for
    pub domain: String,
    /// Organization entry for cert gen
    pub organization: String,
    /// Country entry for cert gen
    pub country: String,
    /// PFX password
    pub pfx_pass: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CertEntry<T> {
    /// tag number
    pub tag: u32,
    /// ca cert
    pub ca_cert: Option<T>,
    /// private key
    pub private_key: Option<T>,
    /// pub key
    pub pub_key: Option<T>,
}

/// Config that can be installed locally
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    /// URL of the Pyrinas server to connect to.
    /// For example: pyrinas-admin.yourdomain.com
    pub url: String,
    /// Determines secure connection or not
    pub secure: bool,
    /// Authentication key. This is the same key set in
    /// the Pyrinas config.toml
    pub authkey: String,
    /// Server cert configuration
    pub cert: CertConfig,
    /// Alternative Certs to program to provisioned devices
    pub alts: Option<Vec<CertEntry<String>>>,
}

/// Configuration related commands
#[derive(Parser, Debug, Serialize, Deserialize)]
#[clap(version)]
pub struct ConfigCmd {
    #[clap(subcommand)]
    pub subcmd: ConfigSubCommand,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[clap(version)]
pub enum ConfigSubCommand {
    Show(Show),
    Init,
}

/// Show current configuration
#[derive(Parser, Debug, Serialize, Deserialize)]
#[clap(version)]
pub struct Show {}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAManifest {
    pub version: OTAPackageVersion,
    pub file: String,
    pub force: bool,
}

// pub fn get_git_describe() -> Result<String, Error> {
//     // Expected output 0.2.1-19-g09db6ef-dirty

//     // Get git describe output
//     let out = Command::new("git")
//         .args(&["describe", "--dirty", "--always", "--long"])
//         .output()?;

//     let err = std::str::from_utf8(&out.stderr)?;
//     let out = std::str::from_utf8(&out.stdout)?;

//     // Return error if not blank
//     if err != "" {
//         return Err(anyhow!("Git error. Err: {}", err));
//     }

//     // Convert it to String
//     Ok(out.to_string())
// }

pub fn get_socket(config: &Config) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, Error> {
    if !config.secure {
        println!("WARNING! Not using secure web socket connection!");
    }

    // String of full URL
    let full_uri = format!(
        "ws{}://{}/socket",
        match config.secure {
            true => "s",
            false => "",
        },
        config.url
    );

    log::info!("{}", full_uri);

    /* Add ApiKey */
    let mut request = full_uri.into_client_request()?;
    request
        .headers_mut()
        .append("ApiKey", config.authkey.clone().parse().unwrap());

    // Connect to TCP based WS socket
    let (socket, _response) = tungstenite::connect(request)?;

    // Return this guy
    Ok(socket)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup() {
        INIT.call_once(|| env_logger::init());
    }

    #[test]
    fn get_ota_package_version_success_with_dirty() {
        // Log setup
        setup();

        let ver = "0.2.1-19-g09db6ef-dirty";

        let res = git::get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_ok());

        let (package_ver, dirty) = res.unwrap();

        // Confirm it's dirty
        assert!(dirty);

        // confirm the version is correct
        assert_eq!(
            package_ver,
            OTAPackageVersion {
                major: 0,
                minor: 2,
                patch: 1,
                commit: 19,
                hash: [
                    'g' as u8, '0' as u8, '9' as u8, 'd' as u8, 'b' as u8, '6' as u8, 'e' as u8,
                    'f' as u8
                ]
            }
        )
    }

    #[test]
    fn get_ota_package_version_success_clean() {
        // Log setup
        setup();

        let ver = "0.2.1-19-g09db6ef";

        let res = git::get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_ok());

        let (package_ver, dirty) = res.unwrap();

        // Confirm it's dirty
        assert!(!dirty);

        // confirm the version is correct
        assert_eq!(
            package_ver,
            OTAPackageVersion {
                major: 0,
                minor: 2,
                patch: 1,
                commit: 19,
                hash: [
                    'g' as u8, '0' as u8, '9' as u8, 'd' as u8, 'b' as u8, '6' as u8, 'e' as u8,
                    'f' as u8
                ]
            }
        )
    }

    #[test]
    fn get_ota_package_version_failure_dirty() {
        // Log setup
        setup();

        let ver = "0.2.1-g09db6ef-dirty";

        let res = git::get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_err());
    }

    // TODO: address this
    #[allow(dead_code)]
    fn get_git_describe_success() {
        // Log setup
        setup();

        let res = git::get_git_describe();

        // Make sure it processed ok
        assert!(res.is_ok());

        log::info!("res: {}", res.unwrap());
    }
}
