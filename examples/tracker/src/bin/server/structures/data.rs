use crate::structures::influx;
use chrono::{serde::ts_milliseconds::deserialize as from_ts, DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerGpsReport {
    pub lng: f32,
    pub lat: f32,
    pub acc: f32,
    pub alt: f32,
    pub spd: f32,
    pub hdg: f32,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerGpsReport {
    pub fn to_influx(&self, id: &str) -> influx::TrackerGpsInfluxReport {
        // Return new data structure that's friendly with Influx
        influx::TrackerGpsInfluxReport {
            time: self.ts,
            id: id.to_string(),
            lng: self.lng,
            lat: self.lat,
            acc: self.acc,
            alt: self.alt,
            spd: self.spd,
            hdg: self.hdg,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerNetworkReport {
    pub rsrp: u16,
    pub area: u32,
    pub mnc: u32,
    pub mcc: u32,
    pub cell: u32,
    pub ip: String,
    pub band: u16,
    pub m_gps: u16,
    pub m_lte: u16,
    pub m_nb: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerSimReport {
    pub iccid: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerInfoReport {
    #[serde(alias = "modv")]
    pub mod_version: String,
    #[serde(alias = "brdv")]
    pub brd_version: String,
    #[serde(alias = "appv")]
    pub app_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerDeviceReport {
    pub vbat: u16,
    pub nw: TrackerNetworkReport,
    pub sim: TrackerSimReport,
    pub inf: TrackerInfoReport,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerDeviceReport {
    pub fn to_influx(&self, id: &str) -> influx::TrackerDeviceInfluxReport {
        // Return new data structure that's friendly with Influx
        influx::TrackerDeviceInfluxReport {
            time: self.ts,
            id: id.to_string(),
            rsrp: self.nw.rsrp,
            area: self.nw.area,
            mnc: self.nw.mnc,
            mcc: self.nw.mcc,
            cell: self.nw.cell,
            ip: self.nw.ip.clone(),
            band: self.nw.band,
            mode_gps: self.nw.m_gps,
            mode_lte: self.nw.m_lte,
            mode_nbiot: self.nw.m_nb,
            iccid: self.sim.iccid.clone(),
            modem_version: self.inf.mod_version.clone(),
            board: self.inf.brd_version.clone(),
            app_version: self.inf.app_version.clone(),
            vbat: self.vbat,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerAccelReport {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(deserialize_with = "from_ts")]
    pub ts: DateTime<Utc>,
}

impl TrackerAccelReport {
    pub fn to_influx(&self, id: &str) -> influx::TrackerAccelInfluxReport {
        // Return new data structure that's friendly with Influx
        influx::TrackerAccelInfluxReport {
            time: self.ts,
            id: id.to_string(),
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}
