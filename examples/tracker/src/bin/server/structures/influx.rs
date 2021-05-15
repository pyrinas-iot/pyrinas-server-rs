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
pub struct TrackerRoamInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub rsrp: u16,
    pub area: u32,
    pub mccmnc: u32,
    pub cell: u32,
    pub ip: String,
}

#[derive(Debug, InfluxDbWriteable, Clone)]
pub struct TrackerBattInfluxReport {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub id: String,
    pub val: u32,
}
