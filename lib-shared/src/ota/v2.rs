use std::{fmt, str};

use minicbor::{Decode, Encode};

use serde::{Deserialize, Serialize};
use serde_repr::*;

use chrono::{DateTime, Utc};

use super::OTAPackageVersion;

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum OTADeviceType {
    /// Hub/cellular device
    Cellular,
    /// Bluetooth device
    Bluetooth,
}

impl fmt::Display for OTADeviceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            OTADeviceType::Cellular => "cellular",
            OTADeviceType::Bluetooth => "bluetooth",
        };

        write!(f, "{}", text)
    }
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAPackage {
    /// Identifier for OTA package
    pub id: String,
    /// Version information
    pub version: OTAPackageVersion,
    /// All files associated with this package
    pub file: Option<OTAImageData>,
    /// Size of image
    pub size: usize,
    /// Timestamp for tracking when this was added
    pub date_added: DateTime<Utc>,
}

// Struct that gets serialized for OTA support
#[derive(Debug, Encode, Decode, Serialize, Deserialize, Clone, Default)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[n(4)]
    pub device_uid: Option<String>,
}

impl fmt::Display for OTAPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version,)
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum OTAImageType {
    /// This is a main firmware image
    Primary = 1 << 0,
    /// This is a secondary file. Could be used as a pre-image download
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OTAImageData {
    pub data: Vec<u8>,
    pub image_type: OTAImageType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OTAUpdate {
    /// Unique ID of the device this may get sent to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_uid: Option<String>,
    /// Data on the update itself
    pub package: Option<OTAPackage>,
}
