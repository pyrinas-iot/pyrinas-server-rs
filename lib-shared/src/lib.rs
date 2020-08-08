use bytes::Bytes;
use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use influxdb::Timestamp;
use serde_derive::{Deserialize, Serialize};
use serde_repr::*;
use tokio::sync::mpsc::Sender;

// TODO: confirm the name works for each
// Matches `pyrinas_cloud_telemetry_type_t` in `pyrinas_cloud.h`
// Note: the enum indexes match the entry order below. i.e. the order counts!
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryData {
    version: String,
    rsrp: Option<u32>,        // Won't always have rsrp (hub only)
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
}

#[derive(Debug, InfluxDbWriteable)]
pub struct InfluxTelemetryData {
    version: String,
    rsrp: Option<u32>,        // Won't always have rsrp (hub only)
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
    #[tag]
    id: String,    // Typically not sent as it's included in the MQTT topic
    time: DateTime<Utc>,      // Only used for insertting data into Influx DB
}

impl TelemetryData {
    pub fn to_influx_data(&self, uid: String) -> InfluxTelemetryData {
        InfluxTelemetryData {
            version: self.version.clone(),
            rsrp: self.rsrp,
            rssi_hub: self.rssi_hub,
            rssi_client: self.rssi_client,
            id: uid,
            time: Timestamp::Now.into(),
        }
    }
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OTAPackage {
    pub version: String,
    pub host: String,
    pub file: String,
    pub force: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewOta {
    pub uid: String,
    pub package: OTAPackage,
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

#[derive(Debug, Clone)]
pub enum Event {
    NewRunner { name: String, sender: Sender<Event> },
    OtaNewPackage { uid: String, package: OTAPackage },
    OtaRequest { uid: String, msg: OtaRequest },
    OtaResponse { uid: String, package: OTAPackage },
    TelemetryData { uid: String, msg: TelemetryData },
    ApplicationData { uid: String, msg: Bytes },
    OtaDeletePackage { uid: String, package: OTAPackage },
    SledFlush,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
