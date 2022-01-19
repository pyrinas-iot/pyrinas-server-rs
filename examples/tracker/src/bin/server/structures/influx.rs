use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct TrackerGpsInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub lng: f32,
    pub lat: f32,
    pub acc: f32,
    pub alt: f32,
    pub spd: f32,
    pub hdg: f32,
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct TrackerDeviceInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub rsrp: u16,
    pub area: u32,
    pub mnc: u32,
    pub mcc: u32,
    pub cell: u32,
    pub ip: String,
    pub band: u16,
    pub mode_gps: u16,
    pub mode_lte: u16,
    pub mode_nbiot: u16,
    pub iccid: String,
    pub modem_version: String,
    pub board: String,
    pub app_version: String,
    pub vbat: u16,
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct TrackerBattInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub val: u32,
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct TrackerAccelInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
