use serde::Deserialize;
use std::fs;
use toml;

#[derive(Deserialize)]
pub struct Config {
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub tournament_director: String,
    pub server_url: String,
    pub postgres_options: String,
}

pub fn from_file(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(&fs::read_to_string(&path)?)?)
}
