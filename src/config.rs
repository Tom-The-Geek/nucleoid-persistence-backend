use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use rand::Rng;
use rand::distributions::Alphanumeric;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub database_name: String,
    pub api_port: u16,
    pub server_tokens: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let random_token = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        Self {
            database_url: "mongodb://localhost/".to_string(),
            database_name: "nucleoid_players".to_string(),
            api_port: 3030,
            server_tokens: vec![random_token],
        }
    }
}

pub(super) fn load() -> Config {
    let path = Path::new("config.json");
    if path.exists() {
        let mut file = File::open(path).unwrap();
        serde_json::from_reader(&mut file).unwrap()
    } else {
        let config = Config::default();

        let mut file = File::create(path).unwrap();
        serde_json::to_writer_pretty(&mut file, &config).unwrap();

        config
    }
}
