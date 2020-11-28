pub mod ota;

use clap::{crate_version, Clap};
use serde::{Deserialize, Serialize};

/// Add a new OTA package to the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
pub struct OtaAdd {
    /// UID to be directed to
    pub uid: String,
    /// JSON manifest file
    pub manifest: String,
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAManifest {
    pub version: pyrinas_shared::OTAPackageVersion,
    pub file: String,
    pub force: bool,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
