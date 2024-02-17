use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    listen: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: String::from("[::]:9185"),
        }
    }
}
