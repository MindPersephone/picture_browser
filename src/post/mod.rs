use std::{
    fs::{self, File},
    io::Write,
    ops::Sub,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Local};
use log::info;
use rand::{rng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use crate::{error::Error, post::ice_shrimp::send_ice_shrimp_message};

mod ice_shrimp;

pub fn send_message(src_folder: &str) -> Result<(), Error> {
    let config_path = config_path();
    let config = match config_path {
        Ok(p) => Config::load(p)?,
        Err(e) => {
            info!("No config path found using default: {}", e);
            Config::default()
        }
    };
    let last_post = LastPost::load()?;

    if !should_send_message(&config, &last_post, src_folder) {
        return Ok(());
    }

    let message = generate_message(&config, src_folder)?;

    info!("Message: {}", &message);

    LastPost::now().write()?;

    match config.api_type.as_str() {
        "ice_shrimp" => send_ice_shrimp_message(&config, &config.content_warning, &message),
        _ => Err(Error::UnknownApiType(config.api_type.clone())),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    enabled: bool,
    enable_for: Vec<String>,
    display_name: String,
    titles: Vec<String>,
    activities: Vec<String>,
    content_warning: String,
    folder_filters: Vec<Filter>,
    api_type: String,
    api_key: String,
    api_base_url: String,
    repost_timeout: u32,
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
            activities: vec![],
            content_warning: "".to_string(),
            folder_filters: vec![],
            api_type: "unknown".to_string(),
            api_key: "invalid".to_string(),
            api_base_url: "https://example.com/api".to_string(),
            repost_timeout: 30,
        }
    }
}

fn config_path() -> Result<String, Error> {
    match dirs::config_dir() {
        Some(p) => Ok(p
            .join("picture_browser")
            .join("post.json")
            .to_str()
            .unwrap()
            .to_string()),
        None => Err(Error::NoConfigPath),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct LastPost {
    last_post: DateTime<Local>,
}

impl LastPost {
    fn now() -> Self {
        LastPost {
            last_post: Local::now(),
        }
    }

    fn load() -> Result<Option<LastPost>, Error> {
        let maybe_path: Option<PathBuf> =
            dirs::config_dir().map(|p| p.join("picture_browser").join("last_post.json"));
        if maybe_path.is_none() {
            return Ok(None);
        }
        let p = maybe_path.unwrap();

        if !p.exists() {
            return Ok(None);
        }

        let f = File::open(p)?;
        let result: LastPost = serde_json::from_reader(f)?;
        return Ok(Some(result));
    }

    fn write(&self) -> Result<(), Error> {
        let maybe_path: Option<PathBuf> =
            dirs::config_dir().map(|p| p.join("picture_browser").join("last_post.json"));
        if maybe_path.is_none() {
            return Err(Error::NoConfigPath);
        }
        let p = maybe_path.unwrap();
        // the path can not be at the root of the drive, we've added a
        // folder and a file to it, so this unwrap is safe
        if !p.parent().unwrap().exists() {
            fs::create_dir_all(&p.parent().unwrap())?;
        }

        let payload = serde_json::to_string_pretty(self)?;
        let mut f = File::create(p)?;

        write!(f, "{}", payload)?;

        Ok(())
    }
}

fn should_send_message(config: &Config, last_post: &Option<LastPost>, src_folder: &str) -> bool {
    if !config.enabled {
        return false;
    }

    if !config.enable_for.iter().any(|p| src_folder.starts_with(p)) {
        return false;
    }

    // there has been no last post file created so we can assume we haven't posted yet
    if last_post.is_some()
        && Local::now().sub(last_post.unwrap().last_post)
            <= Duration::minutes(config.repost_timeout as i64)
    {
        return false;
    }

    true
}

fn generate_message(config: &Config, src_folder: &str) -> Result<String, Error> {
    let filtered_folder = filter_folder(&config, src_folder);

    let title = select_field(&config.titles);
    let activity = select_field(&config.activities);

    Ok(format!(
        "{} {} is {} {} using picture browser",
        title, config.display_name, activity, filtered_folder
    ))
}

fn filter_folder(config: &Config, src_folder: &str) -> String {
    let mut result = src_folder.to_string();

    for filter in &config.folder_filters {
        result = result.replace(&filter.find, &filter.replace);
    }

    result
}

fn select_field(list: &Vec<String>) -> &str {
    list.choose(&mut rng()).map(|s| s.as_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use std::ops::Sub;

    use chrono::{Local, TimeDelta};

    use crate::post::{should_send_message, Config, LastPost};

    #[test]
    fn config_not_enabled() {
        let config = Config {
            enabled: false,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            ..Default::default()
        };

        let last_post = LastPost {
            last_post: Local::now().sub(TimeDelta::minutes(31)),
        };

        let test_path = "/home/picture/somewhere/something";
        assert!(!should_send_message(&config, &Some(last_post), test_path));
    }

    #[test]
    fn config_not_path() {
        let config = Config {
            enabled: true,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            ..Default::default()
        };

        let last_post = LastPost {
            last_post: Local::now().sub(TimeDelta::minutes(31)),
        };

        let test_path = "/home/fred/somewhere/something";
        assert!(!should_send_message(&config, &Some(last_post), test_path));
    }

    #[test]
    fn not_yet_timed_out() {
        let config = Config {
            enabled: true,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            repost_timeout: 30,
            ..Default::default()
        };

        let last_post = LastPost {
            last_post: Local::now().sub(TimeDelta::minutes(25)),
        };

        let test_path = "/home/picture/somewhere/something";
        assert!(!should_send_message(&config, &Some(last_post), test_path));
    }

    #[test]
    fn allowed_to_post() {
        let config = Config {
            enabled: true,
            enable_for: vec!["/home/picture/somewhere".to_string()],
            repost_timeout: 30,
            ..Default::default()
        };

        let last_post = LastPost {
            last_post: Local::now().sub(TimeDelta::minutes(31)),
        };

        let test_path = "/home/picture/somewhere/something";
        assert!(should_send_message(&config, &Some(last_post), test_path));
    }
}
