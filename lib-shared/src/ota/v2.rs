use std::fmt;

use minicbor::{Decode, Encode};

use super::OTAPackageVersion;

// Struct that gets serialized for OTA support
#[derive(Debug, Encode, Decode, Clone)]
#[cbor(map)]
pub struct OTAPackage {
    /// Identifier for OTA package
    #[n(0)]
    pub id: String,
    /// Version information
    #[n(1)]
    pub version: OTAPackageVersion,
    /// All files associated with this package
    #[n(2)]
    pub file: Option<OTAImageData>,
    /// Size of image
    #[n(3)]
    pub size: usize,
    /// Timestamp for tracking when this was added
    #[n(4)]
    pub date_added: String,
}

// Struct that gets serialized for OTA support
#[derive(Debug, Encode, Decode, Clone, Default)]
#[cbor(map)]
pub struct OTADownload {
    /// Start position
    #[n(0)]
    pub start_pos: usize,
    /// End position
    #[n(1)]
    pub end_pos: usize,
    /// Raw data download
    #[cbor(n(2), with = "minicbor::bytes")]
    pub data: Vec<u8>,
    /// Length of data
    #[n(3)]
    pub len: usize,
    /// Unique ID of the device this may get sent to
    #[n(4)]
    pub device_uid: Option<String>,
}

impl fmt::Display for OTAPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version,)
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy)]
#[cbor(index_only)]
pub enum OTAImageType {
    /// This is a main firmware image
    #[n(1)]
    Primary = 1 << 0,
    /// This is a secondary file. Could be used as a pre-image download
    #[n(2)]
    Secondary = 1 << 1,
}

impl fmt::Display for OTAImageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = match self {
            OTAImageType::Primary => "primary",
            OTAImageType::Secondary => "secondary",
        };

        write!(f, "{}", string)
    }
}

/// Conveninet renaming so it's more clear what the Vec<Vec<u8>>
#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct OTAImageData {
    #[cbor(n(0), with = "minicbor::bytes")]
    pub data: Vec<u8>,
    #[n(1)]
    pub image_type: OTAImageType,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct OTAUpdate {
    /// Unique ID of the device this may get sent to
    #[n(0)]
    pub device_uid: Option<String>,
    /// Data on the update itself
    #[n(1)]
    pub package: Option<OTAPackage>,
}
