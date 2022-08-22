use std::str;

use ota::v2::OTAPackage;
use serde::{Deserialize, Serialize};
use serde_repr::*;

use clap::Parser;

// Modules
pub mod ota;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaImageListResponse {
    pub images: Vec<(String, OTAPackage)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaGroupListResponse {
    pub groups: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OtaRequest {
    /// Command type
    pub cmd: OtaRequestCmd,
    /// Optional filename
    pub id: Option<String>,
    /// Start position
    pub start_pos: Option<usize>,
    /// End position
    pub end_pos: Option<usize>,
    // char file[PYRINAS_OTA_PACKAGE_MAX_FILE_PATH_CHARS];
}

// Note: uses special _repr functions for using Enum as int
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Clone, Copy, Default)]
#[repr(u8)]
pub enum OtaRequestCmd {
    #[default]
    Check,
    Done,
    DownloadBytes,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ManagmentDataType {
    Application,
    AddOta,
    RemoveOta,
    LinkOta,
    UnlinkOta,
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
#[derive(Parser, Debug, Serialize, Deserialize)]
#[clap(version)]
pub struct OtaLink {
    /// Device Id
    pub device_id: Option<String>,
    /// Group Id
    pub group_id: Option<String>,
    /// Image id to be directed to
    pub image_id: Option<String>,
}
