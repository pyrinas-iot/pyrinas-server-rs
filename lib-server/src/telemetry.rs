use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};
use minicbor::{Decode, Encode};

// TODO: confirm the name works for each
// Matches `pyrinas_cloud_telemetry_type_t` in `pyrinas_cloud.h`
// Note: the enum indexes match the entry order below. i.e. the order counts!
#[derive(Debug, Encode, Decode, Clone)]
#[cbor(map)]
pub struct TelemetryData {
    #[n(0)]
    version: Option<String>,
    #[n(1)]
    rsrp: Option<u32>, // Won't always have rsrp (hub only)
    #[n(2)]
    rssi_hub: Option<i32>, // Won't always have this guy either
    #[n(3)]
    rssi_client: Option<i32>, // Won't always have this guy either
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct InfluxTelemetryData {
    version: Option<String>,  // Wont always have this guy either.
    rsrp: Option<u32>,        // Won't always have rsrp (hub only)
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
    #[influxdb(tag)]
    id: String, // Typically not sent as it's included in the MQTT topic
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
