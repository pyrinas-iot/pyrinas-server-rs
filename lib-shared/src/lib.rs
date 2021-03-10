pub mod settings;

use chrono::{DateTime, Utc};
use flume::Sender;
use influxdb::{InfluxDbWriteable, ReadQuery, WriteQuery};
use serde::{Deserialize, Serialize};
use serde_repr::*;

// TODO: confirm the name works for each
// Matches `pyrinas_cloud_telemetry_type_t` in `pyrinas_cloud.h`
// Note: the enum indexes match the entry order below. i.e. the order counts!
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryData {
    version: Option<String>,
    rsrp: Option<u32>,        // Won't always have rsrp (hub only)
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct InfluxTelemetryData {
    version: Option<String>,  // Wont always have this guy either.
    rsrp: Option<u32>,        // Won't always have rsrp (hub only)
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
    #[tag]
    id: String,    // Typically not sent as it's included in the MQTT topic
    time: DateTime<Utc>,      // Only used for inserting data into Influx DB
}

impl TelemetryData {
    pub fn to_influx_data(&self, uid: String) -> InfluxTelemetryData {
        // Return new data structure that's friendly with Influx
        InfluxTelemetryData {
            version: self.version.clone(),
            rsrp: self.rsrp,
            rssi_hub: self.rssi_hub,
            rssi_client: self.rssi_client,
            id: uid,
            time: Utc::now(),
        }
    }
}

impl InfluxTelemetryData {
    pub fn to_influx_query(&self, category: String) -> WriteQuery {
        // Create and return query
        let data = self.clone();
        data.into_query(category)
    }
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Clone)]
pub enum Event {
    NewRunner { name: String, sender: Sender<Event> },
    OtaDeletePackage(OtaUpdate),
    OtaNewPackage(OtaUpdate),
    OtaRequest { uid: String, msg: OtaRequest },
    OtaResponse(OtaUpdate),
    ApplicationManagementRequest(ManagementData), // Message sent for configuration of application
    ApplicationManagementResponse(ManagementData), // Reponse from application management portion of the app
    ApplicationRequest(ApplicationData),           // Request/event from a device
    ApplicationResponse(ApplicationData),          // Reponse from other parts of the server
    InfluxDataSave(WriteQuery),                    // Takes a pre-prepared query and executes it
    InfluxDataRequest(ReadQuery), // Takes a pre-prepared query to *read* the database
    InfluxDataResponse,           // Is the response to InfluxDataRequest
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
