use figment::{
    providers::{Env, Serialized},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub listen: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            listen: "[::]:9185".into(),
        }
    }
}

pub fn build_config() -> Config {
    Figment::from(Serialized::defaults(Config::default()))
        .merge(Env::prefixed("VATSIM_EXPORTER_"))
        .extract()
        .unwrap()
}
