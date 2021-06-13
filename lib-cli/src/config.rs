use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unable to get home path")]
    HomeError,
}

pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    // Get the config file from standard location
    let mut config_path = home::home_dir().ok_or(ConfigError::HomeError)?;

    // Append config path to home directory
    config_path.push(".pyrinas");

    // Return it
    Ok(config_path)
}
