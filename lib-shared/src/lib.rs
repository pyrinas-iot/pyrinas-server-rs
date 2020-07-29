use serde_derive::{Deserialize, Serialize};

// TODO: confirm the name works for each
// Matches `pyrinas_cloud_telemetry_type_t` in `pyrinas_cloud.h`
// Note: the enum indexes match the entry order below. i.e. the order counts!
#[derive(Debug, Serialize, Deserialize)]
pub struct Telemetry {
    version: String,
    rsrp: Option<u32>,        // Won't always have rsrp
    rssi_hub: Option<i32>,    // Won't always have this guy either
    rssi_client: Option<i32>, // Won't always have this guy either
}

// Struct that gets serialized for OTA support
#[derive(Debug, Serialize, Deserialize)]
pub struct OTAPackage {
    pub version: String,
    pub host: String,
    pub file: String,
    pub force: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOta {
    pub uid: String,
    pub package: OTAPackage,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
