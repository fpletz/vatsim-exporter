use env_logger::{Builder, Env};
use log::{debug, info};

use metrics::gauge;

use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

mod vatsim;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr_raw = "[::]:9185";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(addr)
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::GAUGE,
            Some(Duration::from_secs(40)),
        )
        .install()
        .expect("failed to install Prometheus recorder");

    let vatsim_client = reqwest::Client::new();
    let mut last_etag: String = String::from("");

    loop {
        debug!("requesting vatsim data");
        let response: reqwest::Response = vatsim_client
            .get("https://data.vatsim.net/v3/vatsim-data.json")
            .header(reqwest::header::IF_NONE_MATCH, last_etag)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        last_etag = String::from(
            response
                .headers()
                .get(reqwest::header::ETAG)
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap(),
        );
        let status = response.status();

        if status == 304 {
            debug!("vatsim data still cached");
            thread::sleep(Duration::from_millis(60000));
            continue;
        }

        let vatsim_data: vatsim::VatsimStatus = response.json().await?;

        info!(
            "new vatsim status data {}",
            vatsim_data.general.update_timestamp
        );

        let mut arr_map: HashMap<&str, u32> = HashMap::new();
        vatsim_data
            .pilots
            .iter()
            .filter_map(|pilot| pilot.flight_plan.as_ref())
            .filter(|fpl| fpl.arrival.len() > 0)
            .for_each(|x| {
                *arr_map.entry(&x.arrival).or_default() += 1;
            });

        for (icao, c) in arr_map {
            gauge!("vatsim_airport_arrivals_current", c as f64, "icao" => String::from(icao), "state" => "online");
        }

        let mut arr_prefile_map: HashMap<&str, u32> = HashMap::new();
        vatsim_data
            .prefiles
            .iter()
            .map(|pf| &pf.flight_plan)
            .filter(|fpl| fpl.arrival.len() > 0)
            .for_each(|x| {
                *arr_prefile_map.entry(&x.arrival).or_default() += 1;
            });

        for (icao, c) in arr_prefile_map {
            gauge!("vatsim_airport_arrivals_current", c as f64, "icao" => String::from(icao), "state" => "prefiled");
        }

        let mut adep_map: HashMap<&str, u32> = HashMap::new();
        vatsim_data
            .pilots
            .iter()
            .filter_map(|pilot| pilot.flight_plan.as_ref())
            .filter(|fpl| fpl.departure.len() > 0)
            .for_each(|x| {
                *adep_map.entry(&x.departure).or_default() += 1;
            });

        for (icao, c) in adep_map {
            gauge!("vatsim_airport_departures_current", c as f64, "icao" => String::from(icao), "state" => "online");
        }

        let mut adep_prefile_map: HashMap<&str, u32> = HashMap::new();
        vatsim_data
            .prefiles
            .iter()
            .map(|pf| &pf.flight_plan)
            .filter(|fpl| fpl.departure.len() > 0)
            .for_each(|x| {
                *adep_prefile_map.entry(&x.departure).or_default() += 1;
            });

        for (icao, c) in adep_prefile_map {
            gauge!("vatsim_airport_departures_current", c as f64, "icao" => String::from(icao), "state" => "prefiled");
        }

        for controller in vatsim_data.controllers {
            gauge!("vatsim_controller_online", 1.0,
              "callsign" => controller.callsign, "cid" => controller.cid.to_string(), "name" => controller.name,
              "facility" => vatsim_data.facilities.iter().filter(|f| f.id == controller.facility).next().unwrap().short.clone()
            );
        }

        for pilot in vatsim_data.pilots {
            gauge!("vatsim_pilot_groundspeed", pilot.groundspeed as f64,
              "callsign" => pilot.callsign.clone(), "cid" => pilot.cid.to_string(), "name" => pilot.name.clone(),
            );
            gauge!("vatsim_pilot_altitude", pilot.altitude as f64,
              "callsign" => pilot.callsign.clone(), "cid" => pilot.cid.to_string(), "name" => pilot.name.clone(),
            );
            gauge!("vatsim_pilot_heading", pilot.heading as f64,
              "callsign" => pilot.callsign.clone(), "cid" => pilot.cid.to_string(), "name" => pilot.name.clone(),
            );
            gauge!("vatsim_pilot_latitude", pilot.latitude,
              "callsign" => pilot.callsign.clone(), "cid" => pilot.cid.to_string(), "name" => pilot.name.clone(),
            );
            gauge!("vatsim_pilot_longitude", pilot.longitude,
              "callsign" => pilot.callsign.clone(), "cid" => pilot.cid.to_string(), "name" => pilot.name.clone(),
            );
        }

        thread::sleep(Duration::from_millis(60000));
    }
}
