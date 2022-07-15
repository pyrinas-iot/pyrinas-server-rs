pub mod v2;

use std::{fmt, str};

use serde::{Deserialize, Serialize};

/// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct OTAPackageVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub commit: u8,
    pub hash: [u8; 8],
}

/// Implents display for package version for easy to_string() calls
impl fmt::Display for OTAPackageVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hash = match str::from_utf8(&self.hash) {
            Ok(h) => h,
            Err(_e) => "unknown",
        };

        write!(
            f,
            "{}.{}.{}-{}-{}",
            self.major, self.minor, self.patch, self.commit, hash
        )
    }
}
