use std::{fmt, str};

use serde::{Deserialize, Serialize};

use super::OTAPackageVersion;

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAPackage {
    pub version: OTAPackageVersion,
    pub host: String,
    pub file: String,
    pub force: bool,
}

impl fmt::Display for OTAPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version,)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaUpdate {
    pub uid: String,
    pub package: Option<OTAPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<u8>>,
}
