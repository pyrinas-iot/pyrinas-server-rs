use minicbor::{Decode, Encode};
use ota::v2::OTAPackage;

use clap::Parser;

// Modules
pub mod ota;

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct OtaImageListResponseEntry {
    #[n(0)]
    pub name: String,
    #[n(1)]
    pub package: OTAPackage,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct OtaImageListResponse {
    #[n(0)]
    pub images: Vec<OtaImageListResponseEntry>,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct OtaGroupListResponse {
    #[n(0)]
    pub groups: Vec<String>,
}

#[derive(Encode, Decode, Debug, Clone, Default)]
#[cbor(map)]
pub struct OtaRequest {
    /// Command type
    #[n(0)]
    pub cmd: OtaRequestCmd,
    /// Optional filename
    #[n(1)]
    pub id: Option<String>,
    /// Start position
    #[n(2)]
    pub start_pos: Option<usize>,
    /// End position
    #[n(3)]
    pub end_pos: Option<usize>,
    // char file[PYRINAS_OTA_PACKAGE_MAX_FILE_PATH_CHARS];
}

// Note: uses special _repr functions for using Enum as int
#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy, Default)]
#[cbor(index_only)]
pub enum OtaRequestCmd {
    #[default]
    #[n(0)]
    Check,
    #[n(1)]
    Done,
    #[n(2)]
    DownloadBytes,
}

#[derive(Decode, Encode, PartialEq, Eq, Debug, Clone, Copy)]
#[cbor(index_only)]
pub enum ManagmentDataType {
    #[n(0)]
    Application,
    #[n(1)]
    AddOta,
    #[n(2)]
    RemoveOta,
    #[n(3)]
    LinkOta,
    #[n(4)]
    UnlinkOta,
    #[n(5)]
    GetGroupList,
    #[n(6)]
    GetImageList,
}

#[derive(Decode, Encode, Debug, Clone)]
#[cbor(map)]
pub struct ManagementData {
    #[n(0)]
    pub cmd: ManagmentDataType,
    #[n(1)]
    pub target: Option<String>,
    #[n(2)]
    pub msg: Vec<u8>,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct PyrinasEventName {
    #[n(0)]
    pub size: u32,
    #[n(1)]
    pub bytes: Vec<u8>,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct PyrinasEventData {
    #[n(0)]
    pub size: u32,
    #[n(1)]
    pub bytes: Vec<u8>,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct PyrinasEvent {
    #[n(0)]
    pub name: PyrinasEventName,
    #[n(1)]
    pub data: PyrinasEventData,
    #[n(2)]
    pub peripheral_addr: Vec<u8>,
    #[n(3)]
    pub central_addr: Vec<u8>,
    #[n(4)]
    pub peripheral_rssi: i8,
    #[n(5)]
    pub central_rssi: i8,
}

#[derive(Encode, Decode, Debug, Clone)]
#[cbor(map)]
pub struct ApplicationData {
    #[n(0)]
    pub uid: String,
    #[n(1)]
    pub target: String,
    #[n(2)]
    pub msg: Vec<u8>,
}

/// Used to associate
#[derive(Parser, Debug, Encode, Decode)]
#[cbor(map)]
pub struct OtaLink {
    /// Device Id
    #[n(0)]
    pub device_id: Option<String>,
    /// Group Id
    #[n(1)]
    pub group_id: Option<String>,
    /// Image id to be directed to
    #[n(2)]
    pub image_id: Option<String>,
}
