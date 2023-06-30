use chrono::Utc;
use futures::lock::Mutex;
use metrics_util::MetricKindMask;
use std::collections::HashMap;
use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use env_logger::{Builder, Env};
use log::{debug, error, info};

use axum::{extract::State, routing::get, Json};
use metrics::{absolute_counter, gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use reqwest::{header, Client, StatusCode};

mod vatsim;
use vatsim::VatsimStatus;

type SharedState = Arc<Mutex<AppState>>;

struct AppState {
    recorder_handle: PrometheusHandle,
    etag: String,
    vatsim_data: Option<VatsimStatus>,
}

async fn fetch_vatsim_metrics(etag: &String) -> (Option<VatsimStatus>, Option<String>) {
    let vatsim_client = Client::new();

    let response_result = vatsim_client
        .get("https://data.vatsim.net/v3/vatsim-data.json")
        .header(header::IF_NONE_MATCH, etag)
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    match response_result {
        Ok(response) => {
            let new_etag = match response.headers().get(header::ETAG) {
                Some(etag_hv) => match etag_hv.to_str() {
                    Ok(etag_str) => Some(String::from(etag_str)),
                    _ => None,
                },
                _ => None,
            };

            if response.status() == 304 {
                (None, new_etag)
            } else {
                match response.json::<VatsimStatus>().await {
                    Ok(data) => {
                        info!("new vatsim status data {}", data.general.update_timestamp);
                        (Some(data), new_etag)
                    }
                    Err(e) => {
                        error!("failed to parse vatsim data JSON: {}", e.to_string());
                        (None, new_etag)
                    }
                }
            }
        }
        Err(e) => {
            error!("fetching vatsim data failed: {}", e.to_string());
            (None, None)
        }
    }
}

async fn update_vatsim_metrics(vatsim_data: &VatsimStatus) {
    let mut arr_map: HashMap<&str, u32> = HashMap::new();
    vatsim_data
        .pilots
        .iter()
        .filter_map(|pilot| pilot.flight_plan.as_ref())
        .filter(|fpl| !fpl.arrival.is_empty())
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
        .filter(|fpl| !fpl.arrival.is_empty())
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
        .filter(|fpl| !fpl.departure.is_empty())
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
        .filter(|fpl| !fpl.departure.is_empty())
        .for_each(|x| {
            *adep_prefile_map.entry(&x.departure).or_default() += 1;
        });

    for (icao, c) in adep_prefile_map {
        gauge!("vatsim_airport_departures_current", c as f64, "icao" => String::from(icao), "state" => "prefiled");
    }

    for controller in &vatsim_data.controllers {
        let time_online = Utc::now() - controller.logon_time;
        absolute_counter!("vatsim_controller_online_seconds_count", time_online.num_seconds() as u64,
          "callsign" => controller.callsign.clone(), "cid" => controller.cid.to_string(), "name" => controller.name.clone(),
          "facility" => vatsim_data.facilities.iter().find(|f| f.id == controller.facility).unwrap().short.clone()
        );
    }

    for pilot in &vatsim_data.pilots {
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
}

async fn get_vatsim_metrics(State(state): State<SharedState>) -> String {
    let mut app_state = state.lock().await;

    if app_state.vatsim_data.is_none()
        || app_state
            .vatsim_data
            .as_ref()
            .unwrap()
            .general
            .update_timestamp
            + chrono::Duration::seconds(40)
            < Utc::now()
    {
        if app_state.vatsim_data.is_some() {
            debug!(
                "trying to fetch new vatsim data: {} vs {}",
                app_state
                    .vatsim_data
                    .as_ref()
                    .unwrap()
                    .general
                    .update_timestamp,
                Utc::now()
            );
        }
        let fetch_results = fetch_vatsim_metrics(&app_state.etag).await;

        match fetch_results.0 {
            Some(new_data) => {
                update_vatsim_metrics(&new_data).await;
                app_state.vatsim_data = Some(new_data);
                app_state.etag = fetch_results.1.unwrap_or(String::from(""));
            }
            _ => {
                debug!("no new vatsim data");
            }
        }
    }

    app_state.recorder_handle.render()
}

async fn get_vatsim_data(
    State(state): State<SharedState>,
) -> Result<Json<VatsimStatus>, StatusCode> {
    let app_state = state.lock().await;
    match &app_state.vatsim_data {
        Some(data) => Ok(Json(data.clone())),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

fn app() -> axum::Router {
    let recorder_handle = PrometheusBuilder::new()
        .idle_timeout(MetricKindMask::ALL, Some(Duration::from_secs(40)))
        .install_recorder()
        .expect("failed to install Prometheus recorder");
    let app_state = AppState {
        recorder_handle,
        etag: String::from(""),
        vatsim_data: None,
    };
    let shared_state = SharedState::new(Mutex::new(app_state));

    axum::Router::new()
        .route("/metrics", get(get_vatsim_metrics))
        .route("/vatsim-data.json", get(get_vatsim_data))
        .with_state(Arc::clone(&shared_state))
}

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr = ("[::]:9185".parse() as Result<SocketAddr, AddrParseError>).unwrap();

    axum::Server::bind(&addr)
        .serve(app().into_make_service())
        .await
        .unwrap()
}
