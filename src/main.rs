use env_logger::{Builder, Env};
use log::{info, warn};

use metrics::{
    decrement_gauge, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
    increment_counter, increment_gauge,
};

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;

use chrono::{DateTime, TimeZone, Utc};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

mod vatsim;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let _log = Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr_raw = "[::]:9185";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(addr)
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::GAUGE,
            Some(Duration::from_secs(20)),
        )
        .install()
        .expect("failed to install Prometheus recorder");

    describe_gauge!("airport_departures_current", "foo");
    describe_gauge!("airport_arrivals_current", "foo");

    let vatsim_client = reqwest::Client::new();
    let mut vatsim_lastupdate: DateTime<Utc> = Utc.timestamp_millis(0);

    loop {
        //println!("{:#?}", vatsim_data);
        let vatsim_data: vatsim::VatsimStatus = vatsim_client
            .get("https://data.vatsim.net/v3/vatsim-data.json")
            .send()
            .await?
            .json()
            .await?;

        if vatsim_data.general.update_timestamp <= vatsim_lastupdate {
            warn!("vatsim data stale");
            thread::sleep(Duration::from_millis(1000));
            continue;
        }
        vatsim_lastupdate = vatsim_data.general.update_timestamp;
        info!("new vatsim status data {}", vatsim_lastupdate);

        let mut arr_map: HashMap<&String, u32> = HashMap::new();
        vatsim_data
            .pilots
            .iter()
            .filter_map(|pilot| pilot.flight_plan.as_ref())
            .filter(|fpl| fpl.arrival.len() > 0)
            .for_each(|x| {
                *arr_map.entry(&x.arrival).or_default() += 1;
            });

        for (icao, c) in &arr_map {
            gauge!("airport_arrivals_current", *c as f64, "icao" => String::from(*icao), "state" => "online");
        }

        let mut arr_prefile_map: HashMap<&String, u32> = HashMap::new();
        vatsim_data
            .prefiles
            .iter()
            .map(|pf| &pf.flight_plan)
            .filter(|fpl| fpl.arrival.len() > 0)
            .for_each(|x| {
                *arr_prefile_map.entry(&x.arrival).or_default() += 1;
            });

        for (icao, c) in &arr_prefile_map {
            gauge!("airport_arrivals_current", *c as f64, "icao" => String::from(*icao), "state" => "prefiled");
        }

        let mut adep_map: HashMap<&String, u32> = HashMap::new();
        vatsim_data
            .pilots
            .iter()
            .filter_map(|pilot| pilot.flight_plan.as_ref())
            .filter(|fpl| fpl.departure.len() > 0)
            .for_each(|x| {
                *adep_map.entry(&x.departure).or_default() += 1;
            });

        for (icao, c) in &adep_map {
            gauge!("airport_departures_current", *c as f64, "icao" => String::from(*icao), "state" => "online");
        }

        let mut adep_prefile_map: HashMap<&String, u32> = HashMap::new();
        vatsim_data
            .prefiles
            .iter()
            .map(|pf| &pf.flight_plan)
            .filter(|fpl| fpl.departure.len() > 0)
            .for_each(|x| {
                *adep_prefile_map.entry(&x.departure).or_default() += 1;
            });

        for (icao, c) in &adep_prefile_map {
            gauge!("airport_departures_current", *c as f64, "icao" => String::from(*icao), "state" => "prefiled");
        }

        thread::sleep(Duration::from_millis(1000));
    }
}
