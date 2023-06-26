use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_timestamp_without_zulu<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    if !s.ends_with('Z') {
        // no ms present :(
        s.push_str(".0Z");
    }
    Utc.datetime_from_str(&s, "%+")
        .map_err(serde::de::Error::custom)
}

pub type Cid = u32;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Atis {
    pub atis_code: Option<String>,
    pub callsign: String,
    pub cid: Cid,
    pub facility: u8,
    pub frequency: String,
    pub last_updated: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_timestamp_without_zulu")]
    pub logon_time: DateTime<Utc>,
    pub name: String,
    pub rating: u8,
    pub server: String,
    pub text_atis: Option<Vec<String>>,
    pub visual_range: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Controller {
    pub callsign: String,
    pub cid: Cid,
    pub facility: u8,
    pub frequency: String,
    pub last_updated: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_timestamp_without_zulu")]
    pub logon_time: DateTime<Utc>,
    pub name: String,
    pub rating: u8,
    pub server: String,
    pub text_atis: Option<Vec<String>>,
    pub visual_range: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Facility {
    pub id: u8,
    pub long: String,
    pub short: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VatsimGeneral {
    pub connected_clients: u32,
    pub reload: u8,
    pub unique_users: u32,
    pub update: String,
    pub update_timestamp: DateTime<Utc>,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PilotRating {
    pub id: u8,
    pub long_name: String,
    pub short_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FlightRule {
    I,
    V,
    S,
    D,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlightPlan {
    pub aircraft: String,
    pub aircraft_faa: String,
    pub aircraft_short: String,
    pub alternative: Option<String>,
    pub altitude: String,
    pub arrival: String,
    pub assigned_transponder: String,
    pub cruise_tas: String,
    pub departure: String,
    pub deptime: String,
    pub enroute_time: String,
    pub flight_rules: FlightRule,
    pub fuel_time: String,
    pub remarks: String,
    pub revision_id: u32,
    pub route: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pilot {
    pub altitude: i32,
    pub callsign: String,
    pub cid: Cid,
    pub flight_plan: Option<FlightPlan>,
    pub groundspeed: i32,
    pub heading: u16,
    pub last_updated: String,
    pub latitude: f64,
    #[serde(deserialize_with = "deserialize_timestamp_without_zulu")]
    pub logon_time: DateTime<Utc>,
    pub longitude: f64,
    pub name: String,
    pub pilot_rating: u8,
    pub qnh_i_hg: f64,
    pub qnh_mb: f64,
    pub server: String,
    pub transponder: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prefile {
    pub callsign: String,
    pub cid: Cid,
    pub flight_plan: FlightPlan,
    pub last_updated: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub client_connections_allowed: bool,
    pub hostname_or_ip: String,
    pub ident: String,
    pub is_sweatbox: bool,
    pub location: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VatsimStatus {
    pub general: VatsimGeneral,
    pub pilots: Vec<Pilot>,
    pub controllers: Vec<Controller>,
    pub atis: Vec<Atis>,
    pub facilities: Vec<Facility>,
    pub pilot_ratings: Vec<PilotRating>,
    pub prefiles: Vec<Prefile>,
    pub servers: Vec<Server>,
}
