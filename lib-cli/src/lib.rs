pub mod certs;
pub mod device;
pub mod ota;

use clap::{crate_version, Clap};
use pyrinas_shared::OtaAssociate;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, io, num, path::PathBuf};

// Getting git repo information
use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use semver::Version;

// Error handling
use thiserror::Error;

// Websocket
use tungstenite::{client::AutoStream, http, http::Request, protocol::WebSocket};

#[derive(Debug, Error)]
pub enum CliError {
    #[error("filesystem error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("git error: {source}")]
    GitError {
        #[from]
        source: git2::Error,
    },

    #[error("git repo not found!")]
    GitNotFound,

    #[error("unable to convert hash")]
    HashError,

    #[error("toml error: {source}")]
    TomlError {
        #[from]
        source: toml::de::Error,
    },

    #[error("unable to get home path")]
    HomeError,

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
        source: semver::SemVerError,
    },

    #[error("parse error: {source}")]
    ParseError {
        #[from]
        source: num::ParseIntError,
    },
}

/// Various commands related to the OTA process
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaCmd {
    #[clap(subcommand)]
    pub subcmd: OtaSubCommand,
}

/// Commands related to certs
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct CertCmd {
    #[clap(subcommand)]
    pub subcmd: CertSubcommand,
}

#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub enum CertSubcommand {
    /// Generate CA cert
    Ca,
    /// Generate server cert
    Server,
    /// Generate device cert
    Device { id: String },
}

#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub enum OtaSubCommand {
    /// Add OTA package
    Add(OtaAdd),
    /// Associate command
    Associate(OtaAssociate),
    /// Remove OTA package
    Remove(OtaRemove),
    /// List groups
    ListGroups,
    /// List images
    ListImages,
}

/// Add a OTA package from the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaAdd {
    /// Force updating in dirty repository
    #[clap(long, short)]
    pub force: bool,
}

/// Remove a OTA package from the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
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

/// Config that can be installed locally
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    /// URL of the Pyrinas server to connect to.
    /// For example: pyrinas-admin.yourdomain.com
    pub url: String,
    /// Authentication key. This is the same key set in
    /// the Pyrinas config.toml
    pub authkey: String,
    /// Server cert configuration
    pub cert: CertConfig,
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
    Init,
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

pub fn get_git_describe() -> Result<String, CliError> {
    let mut path = std::env::current_dir()?;

    let repo: Repository;

    // Recursively go up levels to see if there's a .git folder and then stop
    loop {
        repo = match Repository::open(path.clone()) {
            Ok(repo) => repo,
            Err(_e) => {
                if !path.pop() {
                    return Err(CliError::GitNotFound);
                }

                continue;
            }
        };

        break;
    }

    // Describe options
    let mut opts = DescribeOptions::new();
    let opts = opts.describe_all().describe_tags();

    // Describe format
    let mut desc_format_opts = DescribeFormatOptions::new();
    desc_format_opts
        .always_use_long_format(true)
        .dirty_suffix("-dirty");

    // Describe string!
    let des = repo.describe(&opts)?.format(Some(&desc_format_opts))?;

    Ok(des)
}

// pub fn get_git_describe() -> Result<String, OtaError> {
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

pub fn get_ota_package_version(
    ver: &str,
) -> Result<(pyrinas_shared::OTAPackageVersion, bool), CliError> {
    // Parse the version
    let version = Version::parse(ver)?;

    log::info!("ver: {:?}", version);

    // Then convert it to an OTAPackageVersion
    let dirty = ver.contains("dirty");
    let pre: Vec<&str> = ver.split('-').collect();
    let commit: u8 = pre[1].parse()?;
    let hash: [u8; 8] = get_hash(pre[2].as_bytes().to_vec())?;

    Ok((
        pyrinas_shared::OTAPackageVersion {
            major: version.major as u8,
            minor: version.minor as u8,
            patch: version.patch as u8,
            commit: commit,
            hash: hash,
        },
        dirty,
    ))
}

fn get_hash(v: Vec<u8>) -> Result<[u8; 8], CliError> {
    match v.try_into() {
        Ok(r) => Ok(r),
        Err(_e) => Err(CliError::HashError),
    }
}

pub fn get_socket(config: &Config) -> Result<WebSocket<AutoStream>, CliError> {
    // String of full URL
    let full_uri = format!("wss://{}/socket", config.url);

    // Set up handshake request
    let req = Request::builder()
        .uri(full_uri)
        .header("ApiKey", config.authkey.clone())
        .body(())?;

    // Connect to TCP based WS socket
    // TODO: confirm URL is parsed correctly into tungstenite
    let (socket, _response) = tungstenite::connect(req)?;

    // Return this guy
    Ok(socket)
}

/// Fetch the configuration from the provided folder path
pub fn get_config() -> Result<Config, CliError> {
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
pub fn set_config(init: &Config) -> Result<(), CliError> {
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

pub fn get_config_path() -> Result<PathBuf, CliError> {
    // Get the config file from standard location
    let mut config_path = home::home_dir().ok_or(CliError::HomeError)?;

    // Append config path to home directory
    config_path.push(".pyrinas");

    // Return it
    Ok(config_path)
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

        let res = get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_ok());

        let (package_ver, dirty) = res.unwrap();

        // Confirm it's dirty
        assert!(dirty);

        // confirm the version is correct
        assert_eq!(
            package_ver,
            pyrinas_shared::OTAPackageVersion {
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

        let res = get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_ok());

        let (package_ver, dirty) = res.unwrap();

        // Confirm it's dirty
        assert!(!dirty);

        // confirm the version is correct
        assert_eq!(
            package_ver,
            pyrinas_shared::OTAPackageVersion {
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

        let res = get_ota_package_version(ver);

        // Make sure it processed ok
        assert!(res.is_err());
    }

    #[test]
    fn get_git_describe_success() {
        // Log setup
        setup();

        let res = get_git_describe();

        // Make sure it processed ok
        assert!(res.is_ok());

        log::info!("res: {}", res.unwrap());
    }
}
