use std::str;

use ota::{v2::OTAPackage, OtaVersion};
use serde::{Deserialize, Serialize};
use serde_repr::*;

use clap::{crate_version, Clap};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaRequest {
    pub cmd: OtaRequestCmd,
    pub version: Option<OtaVersion>,
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
