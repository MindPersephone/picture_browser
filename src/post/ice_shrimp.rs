use log::info;
use serde::{Deserialize, Serialize};

use crate::{error::Error, post::Config};

#[derive(Debug, Serialize, Deserialize)]
struct CreateMessagePayload {
    text: String,
    visibility: String,
    cw: String,
}

pub fn send_ice_shrimp_message(
    config: &Config,
    content_warning: &str,
    message: &str,
) -> Result<(), Error> {
    let payload = CreateMessagePayload {
        text: message.to_string(),
        visibility: "public".to_string(),
        cw: content_warning.to_string(),
    };

    let target_url = format!("{}{}", config.api_base_url, "/notes/create");

    info!("sending create note request to: {}", &target_url);
    let client = reqwest::blocking::Client::new();
    let json_payload = serde_json::to_string(&payload)?;
    let result = client
        .post(target_url)
        .header("Authorization", format!("bearer {}", &config.api_key))
        .body(json_payload)
        .send();
    info!("result: {:?}", result);
    info!("body: {}", result.unwrap().text().unwrap());
    Ok(())
}
