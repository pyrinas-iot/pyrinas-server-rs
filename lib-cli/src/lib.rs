pub mod ota;

use anyhow::anyhow;
use clap::{crate_version, Clap};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, path::PathBuf};

// Getting git repo information
use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use semver::Version;

// Websocket
use tungstenite::{client::AutoStream, http::Request, protocol::WebSocket};

/// Various commands related to the OTA process
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaCmd {
    #[clap(subcommand)]
    pub subcmd: OtaSubCommand,
}

#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub enum OtaSubCommand {
    /// Add OTA package
    Add(OtaAdd),
    /// Remove OTA package
    Remove(OtaRemove),
}

/// Add a new OTA package to the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaAdd {
    /// UID to be directed to
    pub uid: String,
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
    Init(Config),
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

pub fn get_git_describe() -> anyhow::Result<String> {
    let mut path = std::env::current_dir()?;

    let repo: Repository;

    // Recursively go up levels to see if there's a .git folder and then stop
    loop {
        repo = match Repository::open(path.clone()) {
            Ok(repo) => repo,
            Err(_e) => {
                if !path.pop() {
                    return Err(anyhow!("Could not find repo!"));
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

pub fn get_ota_package_version(
    ver: &str,
) -> anyhow::Result<(pyrinas_shared::OTAPackageVersion, bool)> {
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

fn get_hash(v: Vec<u8>) -> anyhow::Result<[u8; 8]> {
    match v.try_into() {
        Ok(r) => Ok(r),
        Err(_e) => Err(anyhow!("Unable to convert hash.")),
    }
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
    }
}
