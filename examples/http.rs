use std::collections::HashMap;

use anyhow::Result;
use hyper::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};

use lynx_embedded::{connect_wifi, reqwesp};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Anything {
    pub data: String,
    pub json: Option<Account>,
    pub method: String,
    pub origin: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Account {
    username: String,
    password: String,
}

fn main() -> Result<()> {
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();

    // Configure Wifi
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let mut client = reqwesp::Client::new()?;
    // Endpoint for testing REST requests
    let url = "https://httpbin.org/anything";

    {
        // Send `GET` request
        log::info!("`GET` request");

        let mut req = client.get(url);
        let res = req.send()?;

        log::info!(
            "Status Code: {}, URL: {}, Content-Type: {:?}",
            res.status(),
            res.url(),
            res.header(CONTENT_TYPE.as_str()).unwrap()
        );

        let res_text = res.text()?;
        log::info!("Full Response: {res_text}");
    }

    {
        // Send `POST` request with json body
        log::info!("`POST` request with json body");

        let mut req = client.post(url).json(&Account {
            username: "crab".to_string(),
            password: "ferris".to_string(),
        });
        let res = req.send()?;

        log::info!(
            "Status Code: {}, URL: {}, Content-Type: {:?}",
            res.status(),
            res.url(),
            res.header(CONTENT_TYPE.as_str()).unwrap()
        );

        let res_json: Anything = res.json()?;
        log::info!("json field: {:?}", res_json.json);

        if let Some(acc) = res_json.json {
            log::info!("Sent username: {}", acc.username);
            log::info!("Sent password: {}", acc.password);
        }
    }

    {
        // Send `POST` request with form body
        log::info!("`POST` request with form body");

        let mut params = HashMap::new();
        params.insert("lang", "rust");
        params.insert("pancakes", "🥞");

        let mut req = client.post(url).form(&params);
        let res = req.send()?;

        log::info!(
            "Status Code: {}, URL: {}, Content-Type: {:?}",
            res.status(),
            res.url(),
            res.header(CONTENT_TYPE.as_str()).unwrap()
        );

        let res_text = res.text()?;
        log::info!("Full Response: {res_text}");
    }

    Ok(())
}
