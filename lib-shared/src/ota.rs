pub mod v2;

use std::{fmt, str};

use minicbor::{Decode, Encode};

/// Struct that gets serialized for OTA support
#[derive(Debug, Encode, Decode, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct OTAPackageVersion {
    #[n(0)]
    pub major: u8,
    #[n(1)]
    pub minor: u8,
    #[n(2)]
    pub patch: u8,
    #[n(3)]
    pub commit: u8,
    #[n(4)]
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
