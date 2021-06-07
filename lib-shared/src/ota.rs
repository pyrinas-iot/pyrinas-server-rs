pub mod v1;
pub mod v2;

use std::{convert::TryFrom, fmt, str};

use serde::{Deserialize, Serialize};
use serde_repr::*;

/// Determine which OTAPackage being used
#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum OtaVersion {
    V1 = 1,
    V2 = 2,
}

impl TryFrom<u8> for OtaVersion {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == OtaVersion::V1 as u8 => Ok(OtaVersion::V1),
            x if x == OtaVersion::V2 as u8 => Ok(OtaVersion::V2),
            _ => Err(()),
        }
    }
}

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

/// Used for passing different verioned ota data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OtaUpdateVersioned {
    pub v1: Option<v1::OtaUpdate>,
    pub v2: Option<v2::OtaUpdate>,
}
