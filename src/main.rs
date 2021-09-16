//! Tasmota-querying thingie. I'm sorry if you use this, it'll be terrible for sure.

#![warn(missing_docs)]
// used in structs for the regex
#[macro_use] extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod structs;

use regex::bytes::Regex;
use std::time::Duration;
use ipnet::Ipv4Net;

use crate::structs::*;

/// Represents configuration of the app
#[derive(Debug, Clone)]
pub struct TasmotaConfig {
    /// username for access
    pub username: String,
    /// password for access
    pub password: Option<String>,
}

impl TasmotaConfig {
    pub fn new(username: &str, password: Option<&str>) -> Self {
        TasmotaConfig {
            username: username.to_string(),
            password: password.map(str::to_string),
        }
    }
}

/// Queries the home page of the device and looks for the version string to confirm if it's a Tasmota device.
pub async fn check_is_tasmota(device: &TasmotaDevice, client: &reqwest::Client) -> bool {
    let response = crate::get_device_uri(device, &client, String::from("/")).await;
    // eprintln!("check_is_tasmota debug: {:?}", &response);
    // Check for this Tasmota 9.5.0.8 by Theo Arends
    lazy_static! {
        static ref RE: Regex = match Regex::new(r".*Tasmota \d\.\d\.(\d|\d\.\d) by Theo Arends.*") {
            Ok(value) => value,
            Err(error) => panic!("ugh {:?}", error)
        };
    }
    match response{
        Err(error) => {
            if error.is_timeout() {
                debug!("Timed out connecting to {:?}", device.ip);
                false
            } else {
                error!("Failed to query {}: {:?}", device.ip, error);
                false
            }
        },
        Ok(response) => {
            warn!("Successfully connected to {}", device.ip);
            let result_bytes = response.bytes().await.unwrap();
            let result = RE.is_match(&result_bytes);
            debug!("regex result: {:?}", result);
            if !result {
                debug!("{:?}", &result_bytes);
            }
            result
        }
    }
}

/// does a GET request against http://`device.ip``uri`
pub async fn get_device_uri(
    device: &TasmotaDevice,
    client: &reqwest::Client,
    uri: String,
    ) ->
    Result<reqwest::Response, reqwest::Error> {
    // debug!("uri: {:?}", uri);
    let full_url = format!("http://{}{}", device.ip, uri);
    let username = &device.username.to_owned();
    let password = &device.password.to_owned();
    match password {
        Some(password) => {
         client.get(full_url)
            .basic_auth(
                username,
                Some(password))
            .send().await
            },
        None => {
            client.get(full_url)
            .basic_auth(
                username, Some(""))
                .send().await
        }
    }

}

/// hits the `cmnd` endpoint for a given device
pub async fn get_cmnd(
    device: &TasmotaDevice,
    client: &reqwest::Client, cmnd: &str) ->
    Result<reqwest::Response, reqwest::Error> {
    let uri = format!("/cm?cmnd={}", cmnd).clone();
    get_device_uri(device, client, uri).await
}

/// Returns a configured [reqwest::Client]
pub fn get_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::new(2,0))
        .build()
        .unwrap()
}

/// Does the checking and pulling thing, returns a [TasmotaDevice] object if it succeeds..
pub async fn check_host(
    ip: std::net::IpAddr,
    config: TasmotaConfig,

    ) -> Result<TasmotaDevice, String> {

    let client = get_client();

    info!("Checking {}", ip);
        let password = Some(config.password.unwrap().to_string());
        let device = crate::structs::TasmotaDevice::new(
            ip,
            &config.username,
            &password,
        );

        let is_tasmota: bool = check_is_tasmota(&device, &client).await.to_owned();
        if !is_tasmota {
            let result = format!("{} Not tasmota, skipping", ip);
            debug!("{}", result);
            return Err(result);
        }

        let fn_result = get_cmnd(&device, &client, "FriendlyName1").await;
        let friendly_name_1 = match fn_result {
            Ok(value) => value,
            Err(error) => {
                error!("{} Error: {:?}", ip, error);
                return Err(format!("{} Error: {:?}", ip, error))
            }
        };
        eprintln!("friendly_name: {:?}", friendly_name_1);
        if friendly_name_1.status() != 200 {
            let result = format!("{} got status {:?} for FriendlyName call, skipping", friendly_name_1.status(), ip);
            debug!("{}", result);
            return Err(result)
        }
        let friendly_json = friendly_name_1.json();
        let friendly_name_1: FriendlyName1 = match friendly_json.await {
            Ok(value) => value,
            Err(error) => panic!("{} Failed to get FriendlyName1: {:?}", ip, error)
        };

        let devicename = match get_cmnd(&device, &client, "DeviceName").await {
            Ok(val) => val,
            Err(error) => {
                let error = format!("Error calling {}, skipping: {:?}", ip, error);
                error!("{}", error);
                return Err(error)
            }
        };
        let devicename: DeviceName = match devicename.json().await {
            Ok(value) => value,
            Err(error) => {
                let error = format!("Failed to JSON decode for {}: {:?}", ip, error);
                error!("{}", error);
                return Err(error)
            }
        };

        debug!("devicename: \t{:?}", devicename.device_name);
        debug!("friendly_name_1: \t{:?}", friendly_name_1.friendly_name_1);
        Ok(device)
}

#[async_std::main]
async fn main()  -> std::io::Result<()> {
    pretty_env_logger::init();

    let config_file = String::from("~/.config/tasmota-rs.json");
    let config_filename: String = shellexpand::tilde(&config_file).into_owned();
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name(&config_filename))
        .unwrap();


    let username = match config.get_str("username") {
        Ok(value) => value,
        Err(error) => {
            panic!("Couldn't get username: {:?}", error);
        }
    };
    let password: Option<&str> = match config.get("password") {
        Err(error) => {
            eprintln!("Failed to get password: {:?}", error);
            None
        }
        Ok(value) => value
    };

    eprintln!("Auth: {:?}/{:?}", username, password);

    let config_ip: String = match config.get("ip_range") {
        Err(error) => panic!("Failed to get ip_range from config: {:?}", error),
        Ok(value) => {
            eprintln!("IP config: {}", &value);
            value
        }
    };

    let net: Ipv4Net = config_ip.parse().unwrap();
    // let host = String::from("http://10.0.5.129");
    // println!("!");
     // if we fail at this point we deserve to...


    for ip in net.hosts() {

        let _device = tokio::spawn(
            check_host(
                ip.into(),
                TasmotaConfig::new(&username, password),
            )
        );

    }


    Ok(())
}
