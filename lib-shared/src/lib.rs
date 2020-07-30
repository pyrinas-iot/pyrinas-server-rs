use async_std::sync::Sender;
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

#[derive(Debug, Clone)]
pub enum Event {
    NewRunner {
        name: String,
        sender: Sender<Event>,
    },
    NewOtaPackage {
        uid: String,
        package: OTAPackage,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
