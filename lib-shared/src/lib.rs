use std::{fmt, str};

use serde::{Deserialize, Serialize};
use serde_repr::*;

use clap::{crate_version, Clap};

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct OTAPackageVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub commit: u8,
    pub hash: [u8; 8],
}

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
}

impl fmt::Display for OTAPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version,)
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum OTAImageType {
    /// This is a main firmware image
    Primary,
    /// This is a secondary file. Could be used as a pre-image download
    Secondary,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaImageListResponse {
    pub images: Vec<(String, OTAPackage)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaGroupListResponse {
    pub groups: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct OtaRequest {
    pub cmd: OtaRequestCmd,
}

// Note: uses special _repr functions for using Enum as int
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum OtaRequestCmd {
    Check,
    Done,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ManagmentDataType {
    Application,
    AddOta,
    RemoveOta,
    Associate,
    Dissociate,
    GetGroupList,
    GetImageList,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagementData {
    pub cmd: ManagmentDataType,
    pub target: Option<String>,
    pub msg: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PyrinasEventName {
    pub size: u32,
    pub bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PyrinasEventData {
    pub size: u32,
    pub bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PyrinasEvent {
    pub name: PyrinasEventName,
    pub data: PyrinasEventData,
    pub peripheral_addr: Vec<u8>,
    pub central_addr: Vec<u8>,
    pub peripheral_rssi: i8,
    pub central_rssi: i8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationData {
    pub uid: String,
    pub target: String,
    pub msg: Vec<u8>,
}

/// Used to associate
#[derive(Clap, Debug, Serialize, Deserialize)]
#[clap(version = crate_version!())]
pub struct OtaAssociate {
    /// Device Id
    pub device_id: Option<String>,
    /// Group Id
    pub group_id: Option<String>,
    /// Image id to be directed to
    pub image_id: Option<String>,
}
