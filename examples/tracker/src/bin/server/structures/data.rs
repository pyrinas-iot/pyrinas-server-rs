use crate::structures::influx;
use chrono::{serde::ts_milliseconds::deserialize as from_ts, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerGpsReportData {
    pub lng: f32,
    pub lat: f32,
    pub acc: f32,
    pub alt: f32,
    pub spd: f32,
    pub hdg: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerGpsReport {
    pub v: TrackerGpsReportData,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerGpsReport {
    pub fn to_influx(&self, id: &String) -> influx::TrackerGpsInfluxReport {
        // Return new data structure that's friendly with Influx
        let report = influx::TrackerGpsInfluxReport {
            time: self.ts,
            id: id.clone(),
            lng: self.v.lng,
            lat: self.v.lat,
            acc: self.v.acc,
            alt: self.v.alt,
            spd: self.v.spd,
            hdg: self.v.hdg,
        };

        // Return the influx equivalent
        report
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerRoamReportData {
    pub rsrp: u16,
    pub area: u32,
    pub mccmnc: u32,
    pub cell: u32,
    pub ip: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerRoamReport {
    pub v: TrackerRoamReportData,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerRoamReport {
    pub fn to_influx(&self, id: &String) -> influx::TrackerRoamInfluxReport {
        // Return new data structure that's friendly with Influx
        let report = influx::TrackerRoamInfluxReport {
            time: self.ts,
            id: id.clone(),
            rsrp: self.v.rsrp,
            area: self.v.area,
            mccmnc: self.v.mccmnc,
            cell: self.v.cell,
            ip: self.v.ip.clone(),
        };

        // Return the influx equivalent
        report
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerDeviceReportValue {
    pub band: u16,
    pub nw: String,
    pub iccid: String,
    #[serde(alias = "modV")]
    pub mod_version: String,
    #[serde(alias = "brdV")]
    pub brd_version: String,
    #[serde(alias = "appV")]
    pub app_version: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerDeviceReport {
    pub v: TrackerDeviceReportValue,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerBatteryReport {
    pub v: u32,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerBatteryReport {
    pub fn to_influx(&self, id: &String) -> influx::TrackerBattInfluxReport {
        // Return new data structure that's friendly with Influx
        let report = influx::TrackerBattInfluxReport {
            time: self.ts,
            id: id.clone(),
            val: self.v,
        };

        // Return the influx equivalent
        report
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerAccelData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerAccelReport {
    pub v: TrackerAccelData,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerAccelReport {
    pub fn to_influx(&self, id: &String) -> influx::TrackerAccelInfluxReport {
        // Return new data structure that's friendly with Influx
        let report = influx::TrackerAccelInfluxReport {
            time: self.ts,
            id: id.clone(),
            x: self.v.x,
            y: self.v.y,
            z: self.v.z,
        };

        // Return the influx equivalent
        report
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerBulkReport {
    pub acc: Vec<TrackerAccelReport>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerStateReport {
    pub bat: Option<TrackerBatteryReport>,
    pub dev: Option<TrackerDeviceReport>,
    pub roam: Option<TrackerRoamReport>,
    pub gps: Option<TrackerGpsReport>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerState {
    pub reported: TrackerStateReport,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerPayload {
    pub state: TrackerState,
}
