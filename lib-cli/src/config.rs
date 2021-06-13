use std::{io, path::PathBuf};

use thiserror::Error;

use crate::{Config, ConfigCmd, ConfigSubCommand};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unable to get home path")]
    HomeError,

    #[error("filesystem error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("toml error: {source}")]
    TomlError {
        #[from]
        source: toml::de::Error,
    },
}

pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    // Get the config file from standard location
    let mut config_path = home::home_dir().ok_or(ConfigError::HomeError)?;

    // Append config path to home directory
    config_path.push(".pyrinas");

    // Return it
    Ok(config_path)
}

/// Set config
pub fn set_config(init: &Config) -> Result<(), ConfigError> {
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

/// Fetch the configuration from the provided folder path
pub fn get_config() -> Result<Config, ConfigError> {
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

pub fn process(config: &Config, c: &ConfigCmd) -> Result<(), ConfigError> {
    match c.subcmd {
        ConfigSubCommand::Show(_) => {
            println!("{:?}", config);
        }
        ConfigSubCommand::Init => {
            // Default config (blank)
            let c = Default::default();

            // TODO: migrate config on update..

            // Set the config from init struct
            set_config(&c)?;

            println!("Config successfully added!");
        }
    };

    Ok(())
}
