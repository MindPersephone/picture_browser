use std::{fs::File, path::Path};

use log::info;
use rand::{rng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use crate::{error::Error, post::ice_shrimp::send_ice_shrimp_message};

mod ice_shrimp;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    enabled: bool,
    enable_for: Vec<String>,
    display_name: String,
    titles: Vec<String>,
    content_warning: String,
    folder_filters: Vec<Filter>,
    api_type: String,
    api_key: String,
    api_base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Filter {
    find: String,
    replace: String,
}

impl Config {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<Config, Error> {
        let real_path = path.as_ref();
        if !real_path.exists() {
            info!("Config path does not exist");
            info!("Paste the following to {:?}", real_path);
            let default_config = Config::default();
            let json_config = serde_json::to_string_pretty(&default_config)?;
            info!("{}", json_config);
        }

        let f = File::open(real_path)?;
        Ok(serde_json::from_reader(f)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            enable_for: vec![],
            display_name: "Someone".to_string(),
            titles: vec![],
            content_warning: "".to_string(),
            folder_filters: vec![],
            api_type: "unknown".to_string(),
            api_key: "invalid".to_string(),
            api_base_url: "https://example.com/api".to_string(),
        }
    }
}

pub fn send_message(src_folder: &str) -> Result<(), Error> {
    let config_path = config_path();
    let config = match config_path {
        Ok(p) => Config::load(p)?,
        Err(e) => {
            info!("No config path found using default: {}", e);
            Config::default()
        }
    };

    if !should_send_message(&config, src_folder) {
        return Ok(());
    }

    let message = generate_message(&config, src_folder)?;

    info!("Message: {}", &message);

    match config.api_type.as_str() {
        "ice_shrimp" => send_ice_shrimp_message(&config, &config.content_warning, &message),
        _ => Err(Error::UnknownApiType(config.api_type.clone())),
    }
}

fn config_path() -> Result<String, Error> {
    match dirs::config_dir() {
        Some(p) => Ok(p
            .join("picture_browser_post.json")
            .to_str()
            .unwrap()
            .to_string()),
        None => Err(Error::NoConfigPath),
    }
}

fn should_send_message(config: &Config, src_folder: &str) -> bool {
    config.enabled && config.enable_for.iter().any(|p| src_folder.starts_with(p))
}

fn generate_message(config: &Config, src_folder: &str) -> Result<String, Error> {
    let filtered_folder = filter_folder(&config, src_folder);

    let title = select_title(&config);

    Ok(format!(
        "{} {} is looking at folder {} using picture browser",
        title, config.display_name, filtered_folder
    ))
}

fn filter_folder(config: &Config, src_folder: &str) -> String {
    let mut result = src_folder.to_string();

    for filter in &config.folder_filters {
        result = result.replace(&filter.find, &filter.replace);
    }

    result
}

fn select_title(config: &Config) -> &str {
    config
        .titles
        .choose(&mut rng())
        .map(|s| s.as_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use crate::post::{should_send_message, Config};

    #[test]
    fn config_not_enabled() {
        let config = Config {
            enabled: false,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            ..Default::default()
        };

        let test_path = "/home/picture/somewhere/something";
        assert!(!should_send_message(&config, test_path));
    }

    #[test]
    fn config_not_path() {
        let config = Config {
            enabled: true,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            ..Default::default()
        };

        let test_path = "/home/fred/somewhere/something";
        assert!(!should_send_message(&config, test_path));
    }
}
