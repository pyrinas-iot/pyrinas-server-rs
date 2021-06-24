use std::{fmt, str};

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
pub struct OTAPackageFileInfo {
    /// Image type
    pub image_type: OTAImageType,
    /// Full host path
    pub host: String,
    /// Filename on remote host
    pub file: String,
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAPackage {
    /// Version information
    pub version: OTAPackageVersion,
    /// All files associated with this package
    pub files: Vec<OTAPackageFileInfo>,
    /// Timestamp for tracking when this was added
    pub date_added: Option<DateTime<Utc>>,
}

impl fmt::Display for OTAPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version,)
    }
}

impl Into<Option<super::v1::OTAPackage>> for OTAPackage {
    fn into(self) -> Option<super::v1::OTAPackage> {
        let image = self
            .files
            .iter()
            .find(|x| x.image_type == OTAImageType::Primary);

        // Depending if there's a primary image, organize
        match image {
            Some(i) => Some(super::v1::OTAPackage {
                version: self.version.clone(),
                host: i.host.clone(),
                file: i.file.clone(),
                force: false,
            }),
            None => None,
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy)]
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
pub struct OtaUpdate {
    /// Unique ID of the device this may get sent to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    /// Data on the update itself
    pub package: Option<OTAPackage>,
    /// Optional full OTA image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<OTAImageData>>,
}
