//! Tasmota-querying thingie. I'm sorry if you use this, it'll be terrible for sure.

// used in structs for the regex
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::env;
use std::str::from_utf8;
use std::sync::LazyLock;

use futures::future::join_all;

use ipnet::Ipv4Net;
use regex::bytes::Regex;
use reqwest::{Client, Response};
use std::process::exit;
use std::time::Duration;

use tasmota::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    target: Option<String>,
    #[clap(short, long, value_parser)]
    version_filter: Option<String>,
}

/// Hand it the bytes representation of the webpage, you'll (maybe) get the MAC address back.
pub async fn get_mac_address(device: &TasmotaDevice, client: &Client) -> Option<String> {
    let result = get_device_uri(device, client, String::from("/in")).await;

    match result {
        Ok(res) => {
            let mac_finder = match Regex::new(
                r"([A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2})",
            ) {
                Ok(value) => value,
                Err(error) => panic!("failed to create mac address finder regex: {:?}", error),
            };
            //Ok(String::from("48:3F:DA:45:0F:D0"))

            let resbytes = match res.bytes().await {
                Ok(value) => value,
                Err(error) => {
                    let errmsg = format!("Failed to get main page for {}: {:?}", device.ip, error);
                    error!("{}", errmsg);
                    return None;
                }
            };

            let captures = match mac_finder.captures(&resbytes) {
                Some(value) => value,
                None => {
                    let errmsg = format!(
                        "Failed to find MAC address in page content for {}",
                        device.ip
                    );
                    error!("{}", errmsg);
                    debug!("Page bytes:\n{:?}", &resbytes);
                    return None;
                }
            };
            match captures.get(1) {
                Some(value) => {
                    debug!("mac finder: {:?}", value);
                    Some(from_utf8(value.as_bytes()).unwrap().to_string())
                }
                None => {
                    debug!("Failed to get mac address for device: {}", device);
                    None
                }
            }
        }
        Err(error) => {
            error!("Failed to get mac address: {:?}", error);
            None
        }
    }
}

/// Queries the home page of the device and looks for the version string to confirm if it's a Tasmota device.
pub async fn check_is_tasmota(device: &TasmotaDevice, client: &Client) -> Option<String> {
    // TODO: make this return a semver-ish thing
    let response = get_device_uri(device, client, String::from("/")).await;
    // eprintln!("check_is_tasmota debug: {:?}", &response);
    // Check for this Tasmota 9.5.0.8 by Theo Arends
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        match regex::bytes::Regex::new(
            r"(Tasmota (?P<version>\d+\.\d+\.(\d+|\d+\.\d+)) by Theo Arends)",
        ) {
            Ok(value) => value,
            Err(error) => panic!("failed to create tasmota_finder regex: {:?}", error),
        }
    });
    match response {
        Err(error) => {
            if error.is_timeout() {
                debug!("Timed out connecting to {:?}", device.ip);
                None
            } else if error.is_connect() {
                debug!("Connection error connecting to {:?}", device.ip);
                None
            } else {
                error!("Failed to query {}: {:?}", device.ip, error);
                None
            }
        }

        Ok(response) => {
            debug!("Successfully connected to {}", device.ip);
            let result_bytes = match response.bytes().await {
                Ok(value) => value,
                Err(error) => {
                    error!("Failed to query bytes from {}: {error:?}", device.ip);
                    return None;
                }
            };
            match RE.captures(&result_bytes) {
                Some(version) => {
                    debug!(
                        "Named capture: {:?}",
                        from_utf8(version.name("version").unwrap().as_bytes())
                    );
                    Some(
                        from_utf8(version.name("version").unwrap().as_bytes())
                            .unwrap()
                            .to_string(),
                    )
                }
                None => {
                    debug!("Failed to find version for {:?}", device.ip);
                    None
                }
            }
        }
    }
}

/// does a GET request against http://`device.ip``uri`
pub async fn get_device_uri(
    device: &TasmotaDevice,
    client: &Client,
    uri: String,
) -> Result<Response, reqwest::Error> {
    // debug!("uri: {:?}", uri);
    let full_url = format!("http://{}{}", device.ip, uri);
    let username = &device.username.to_owned();
    let password = &device.password.to_owned();
    let response = match password {
        Some(password) => {
            client
                .get(full_url)
                .basic_auth(username, Some(password))
                .send()
                .await?
        }
        None => {
            client
                .get(full_url)
                .basic_auth(username, Some(""))
                .send()
                .await?
        }
    };
    Ok(response)
}

/// hits the `cmnd` endpoint for a given device
pub async fn get_cmnd(
    device: &TasmotaDevice,
    client: &Client,
    cmnd: &str,
) -> Result<Response, reqwest::Error> {
    let uri = format!("/cm?cmnd={}", cmnd);
    get_device_uri(device, client, uri).await
}

/// Returns a configured [reqwest::Client]
pub fn get_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::new(2, 0))
        .build()
        .expect("Failed to build a client")
}

/// gets the FriendlyName1 value from a device
async fn get_friendlyname(input_device: &TasmotaDevice, client: &Client) -> Option<String> {
    let fn_result = get_cmnd(input_device, client, "FriendlyName1").await;
    let response = match fn_result {
        Ok(value) => value,
        Err(error) => {
            let errmsg = format!(
                "{} Error running FriendlyName1 command: {:?}",
                input_device.ip, error
            );
            error!("{}", errmsg);
            return None;
        }
    };

    debug!("friendly_name: {:?}", response);
    if response.status() != 200 {
        let result = format!(
            "{} got status {:?} for FriendlyName call, skipping",
            response.status(),
            input_device.ip
        );
        debug!("{}", result);
        return None;
    }
    let friendly_json = response.json().await;
    let friendly_name: FriendlyName1 = match friendly_json {
        Ok(value) => {
            // Some(value.friendly_name_1),
            debug!("friendly_json: {:?}", value);
            value
        }
        Err(error) => {
            error!(
                "{} Failed to decode FriendlyName1: {:?}",
                input_device.ip, error
            );
            return None;
        }
    };
    Some(friendly_name.friendly_name_1)
}
/// gets the DeviceNAme value from a device
async fn get_devicename(input_device: &TasmotaDevice, client: &Client) -> Option<String> {
    let fn_result = get_cmnd(input_device, client, "DeviceName").await;
    let response = match fn_result {
        Ok(value) => value,
        Err(error) => {
            let errmsg = format!(
                "{} Error running DeviceName command: {:?}",
                input_device.ip, error
            );
            error!("{}", errmsg);
            return None;
        }
    };
    debug!("device_name response: {:?}", response);
    if response.status() != 200 {
        let result = format!(
            "{} got status {:?} for DeviceName call, skipping",
            response.status(),
            input_device.ip
        );
        debug!("{}", result);
        return None;
    }
    let friendly_json = response.json().await;
    let device_name: DeviceName = match friendly_json {
        Ok(value) => {
            debug!("device_name: {:?}", value);
            value
        }
        Err(error) => {
            error!(
                "{} Failed to decode DeviceName: {:?}",
                input_device.ip, error
            );
            return None;
        }
    };
    Some(device_name.device_name)
}

/// Does the checking and pulling thing, returns a [TasmotaDevice] object if it succeeds..
pub async fn check_host(input_device: &mut TasmotaDevice) -> Result<TasmotaDevice, String> {
    debug!("Checking {}", input_device.ip);

    let client = get_client();

    input_device.version = check_is_tasmota(input_device, &client).await;

    if input_device.version.is_none() {
        let result = format!("{} Not tasmota, skipping", input_device.ip);
        debug!("{}", result);
        return Err(result);
    };
    // debug!("Version: {:?}", version);

    input_device.friendly_name_1 = get_friendlyname(input_device, &client).await;
    input_device.device_name = get_devicename(input_device, &client).await;

    input_device.mac_address = get_mac_address(input_device, &client).await;

    // debug!("mac_address: \t{:?}", input_device.mac_address);
    Ok(input_device.to_owned())
}

fn get_config(target: Option<String>) -> config::Config {
    let config_file = String::from("~/.config/tasmota-rs.json");
    let config_filename: String = shellexpand::tilde(&config_file).into_owned();

    let mut builder = config::Config::builder().add_source(config::File::new(
        &config_filename,
        config::FileFormat::Json,
    ));
    builder = match builder.set_override_option("ip_range", target) {
        Ok(builder) => builder,
        Err(error) => panic!("failed to set ip range: {:?}", error),
    };

    match builder.build() {
        Ok(config) => config,
        Err(error) => panic!(
            "Couldn't load config from {:?}: {:?}",
            config_filename, error
        ),
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    debug!("args: {:?}", args);

    let config = get_config(args.target);

    if let Some(version) = &args.version_filter {
        info!("Filtering on versions containing \"{version}\"");
    }

    let max_threads = config.get_int("max_threads").unwrap_or(i64::MAX);

    let username = match config.get_string("username") {
        Ok(value) => value,
        Err(error) => {
            panic!("Couldn't get username: {:?}", error);
        }
    };
    let password = match config.get_string("password") {
        Err(error) => {
            eprintln!("Failed to get password: {:?}", error);
            None
        }
        Ok(value) => Some(value),
    };

    // eprintln!("Auth: {:?}/{:?}", username, password);

    let config_ip = match config.get_string("ip_range") {
        Err(error) => panic!("Failed to get ip_range from config: {:?}", error),
        Ok(value) => {
            info!("IP config: {}", &value);
            value
        }
    };

    let net: Ipv4Net = match config_ip.parse() {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "Failed to parse input {:?} : {:?}",
                config.get_string("ip_range").unwrap(),
                error
            );
            exit(1);
        }
    };

    let mut tasks: Vec<TasmotaDevice> = vec![];

    for ip in net.hosts() {
        tasks.push(TasmotaDevice::new(
            ip.into(),
            username.to_string(),
            password.clone(),
        ))
    }

    let mut results = Vec::new();

    for chunk in tasks.chunks_mut(max_threads as usize) {
        let runners: Vec<_> = chunk
            .iter_mut()
            .map(|device: &mut TasmotaDevice| async move { check_host(device).await.ok() })
            .collect::<Vec<_>>();

        debug!("Waiting for {} tasks", runners.len());

        results.extend(join_all(runners).await);
    }

    if !results.is_empty() {
        info!("Listing found devices");
        info!("#####################");
        for device in results.into_iter().flatten() {
            if let (Some(filter_vers), Some(vers)) = (&args.version_filter, &device.version) {
                if !vers.contains(filter_vers) {
                    continue;
                }
            }
            info!("{}", device);
        }
    }

    Ok(())
}
