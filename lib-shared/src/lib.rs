use serde::{Deserialize, Serialize};
use serde_repr::*;

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct OTAPackageVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub commit: u8,
    pub hash: [u8; 8],
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAPackage {
    pub version: OTAPackageVersion,
    pub host: String,
    pub file: String,
    pub force: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaUpdate {
    pub uid: String,
    pub package: Option<OTAPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<u8>>,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagementData {
    pub target: ManagmentDataType,
    pub msg: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationData {
    pub uid: String,
    pub target: String,
    pub msg: Vec<u8>,
}
