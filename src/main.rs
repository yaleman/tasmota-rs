//! Tasmota-querying thingie. I'm sorry if you use this, it'll be terrible for sure.

#![warn(missing_docs)]
// used in structs for the regex
#[macro_use] extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::env;

// use std::sync::mpsc::channel;
use rayon::prelude::*;

pub mod structs;

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
    /// Build a new [TasmotaConfig]
    pub fn new(username: &str, password: Option<&str>) -> Self {
        TasmotaConfig {
            username: username.to_string(),
            password: password.map(str::to_string),
        }
    }
}

/// Hand it the bytes representation of the webpage, you'll (maybe) get the MAC address back.
pub fn get_mac_address(
    device: &TasmotaDevice
    ) -> Option<String> {

    let result = get_device_uri(device, &get_client(), String::from("/in"));

    match result {
        Ok(res) => {
            let mac_finder = match Regex::new(r"([A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2}:[A-F0-9]{2})"){
                Ok(value) => value,
                Err(error) => panic!("failed to create mac address finder regex: {:?}", error)
            };
            //Ok(String::from("48:3F:DA:45:0F:D0"))

            let resbytes = match res.bytes() {
                Ok(value) => value,
                Err(error) => {
                    let errmsg = format!("Failed to get main page for {}: {:?}", device.ip, error);
                    error!("{}", errmsg);
                    return None
                }
            };

            let captures = match mac_finder.captures(&resbytes) {
                Some(value) => value,
                None => {
                    let errmsg = format!("Failed to find MAC address in page content for {}", device.ip);
                    error!("{}", errmsg);
                    debug!("Page bytes:\n{:?}", &resbytes);
                    return None
                }
            };
            use std::str::from_utf8;
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
        },
        Err(error) => {
            error!("Failed to get mac address: {:?}", error);
            None
        }

    }

}

/// Queries the home page of the device and looks for the version string to confirm if it's a Tasmota device.
pub fn check_is_tasmota(
    device: &TasmotaDevice,
    client: &reqwest::blocking::Client
) -> bool {
    let response = crate::get_device_uri(device, &client, String::from("/"));
    // eprintln!("check_is_tasmota debug: {:?}", &response);
    // Check for this Tasmota 9.5.0.8 by Theo Arends
    lazy_static! {
        static ref RE: Regex = match Regex::new(r".*Tasmota \d\.\d\.(\d|\d\.\d) by Theo Arends.*") {
            Ok(value) => value,
            Err(error) => panic!("failed to create tasmota_finder regex: {:?}", error)
        };
    }
    match response{
        Err(error) => {
            if error.is_timeout() {
                debug!("Timed out connecting to {:?}", device.ip);
                false
            } else if error.is_connect() {
                debug!("Connection error connecting to {:?}", device.ip);
                false
            } else {
                error!("Failed to query {}: {:?}", device.ip, error);
                false
            }
        },
        Ok(response) => {
            warn!("Successfully connected to {}", device.ip);
            let result_bytes = response.bytes().unwrap();
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
pub fn get_device_uri(
    device: &TasmotaDevice,
    client: &reqwest::blocking::Client,
    uri: String,
    ) ->
    Result<reqwest::blocking::Response, reqwest::Error> {
    // debug!("uri: {:?}", uri);
    let full_url = format!("http://{}{}", device.ip, uri);
    let username = &device.username.to_owned();
    let password = &device.password.to_owned();
    let response = match password {
        Some(password) => {
         client.get(full_url)
            .basic_auth(
                username,
                Some(password))
            .send()?
            },
        None => {
            client.get(full_url)
            .basic_auth(
                username, Some(""))
                .send()?
        }
    };
    Ok(response)

}

/// hits the `cmnd` endpoint for a given device
pub fn get_cmnd(
    device: &TasmotaDevice,
    client: &reqwest::blocking::Client, cmnd: &str) ->
    Result<reqwest::blocking::Response, reqwest::Error> {
    let uri = format!("/cm?cmnd={}", cmnd);
    get_device_uri(device, client, uri)
}

/// Returns a configured [reqwest::blocking::Client]
pub fn get_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::new(2,0))
        .build()
        .unwrap()
}

/// gets the FriendlyName1 value from a device
fn get_friendlyname(input_device: &TasmotaDevice) -> Option<String> {
    let fn_result = get_cmnd(&input_device, &get_client(), "FriendlyName1");
    let response = match fn_result {
        Ok(value) => value,
        Err(error) => {
            let errmsg = format!("{} Error running FriendlyName1 command: {:?}", input_device.ip, error);
            error!("{}", errmsg);
            return None
        }
    };
    debug!("friendly_name: {:?}", response);
    if response.status() != 200 {
        let result = format!("{} got status {:?} for FriendlyName call, skipping", response.status(), input_device.ip);
        debug!("{}", result);
        return None
    }
    let friendly_json = response.json();
    let friendly_name: FriendlyName1 = match friendly_json {
        Ok(value) => {
            // Some(value.friendly_name_1),
            debug!("friendly_json: {:?}", value);
            value
        }
        Err(error) => {
            error!("{} Failed to decode FriendlyName1: {:?}", input_device.ip, error);
            return None
        }
    };
    Some(friendly_name.friendly_name_1)
}
/// gets the DeviceNAme value from a device
fn get_devicename(input_device: &TasmotaDevice) -> Option<String> {
    let fn_result = get_cmnd(&input_device, &get_client(), "DeviceName");
    let response = match fn_result {
        Ok(value) => value,
        Err(error) => {
            let errmsg = format!("{} Error running DeviceName command: {:?}", input_device.ip, error);
            error!("{}", errmsg);
            return None
        }
    };
    debug!("device_name response: {:?}", response);
    if response.status() != 200 {
        let result = format!("{} got status {:?} for DeviceName call, skipping", response.status(), input_device.ip);
        debug!("{}", result);
        return None
    }
    let friendly_json = response.json();
    let device_name: DeviceName = match friendly_json {
        Ok(value) => {
            debug!("device_name: {:?}", value);
            value
        }
        Err(error) => {
            error!("{} Failed to decode DeviceName: {:?}", input_device.ip, error);
            return None
        }
    };
    Some(device_name.device_name)
}

/// Does the checking and pulling thing, returns a [TasmotaDevice] object if it succeeds..
pub fn check_host(
    mut input_device: TasmotaDevice,
    ) -> Result<TasmotaDevice, String> {

    let client = get_client();

    info!("Checking {}", input_device.ip);

        let is_tasmota: bool = check_is_tasmota(&input_device, &client);
        if !is_tasmota {
            let result = format!("{} Not tasmota, skipping", input_device.ip);
            debug!("{}", result);
            return Err(result);
        }

        input_device.friendly_name_1 = get_friendlyname(&input_device);
        input_device.device_name = get_devicename(&input_device);

        input_device.mac_address = get_mac_address(&input_device,);

        debug!("mac_address: \t{:?}", input_device.mac_address);

        debug!("friendly_name_1: \t{:?}", input_device.friendly_name_1);
        debug!("devicename: \t{:?}", input_device.device_name);
        Ok(input_device)
}

fn get_config() -> config::Config {
    let config_file = String::from("~/.config/tasmota-rs.json");
    let config_filename: String = shellexpand::tilde(&config_file).into_owned();
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name(&config_filename))
        .unwrap();
    config
}

fn main()  -> std::io::Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init_timed();
    let config = get_config();

    match config.get_int("max_threads") {
        Ok(threads) => {
            info!("Setting max threads to: {}", threads);
            rayon::ThreadPoolBuilder::new().num_threads(threads as usize).build_global().unwrap();
        },
        Err(error) => {
            debug!("Failed to read threads from config: {:?}", error);
        }
    }




    let username = match config.get_str("username") {
        Ok(value) => value,
        Err(error) => {
            panic!("Couldn't get username: {:?}", error);
        }
    };
    let password = match config.get_str("password") {
        Err(error) => {
            eprintln!("Failed to get password: {:?}", error);
            None
        }
        Ok(value) => Some(value)
    };

    // eprintln!("Auth: {:?}/{:?}", username, password);

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

    let mut tasks: Vec<TasmotaDevice> = [].to_vec();

    for ip in net.hosts() {
        tasks.push(
            TasmotaDevice::new(
                ip.into(),
                username.to_string(),
                password.clone(),
            )
        )
    }

    debug!("Made vec of tasks: {:?}", tasks);
    let taskrunner = tasks.into_par_iter()
        .map(check_host);
    let results: Vec<Result<TasmotaDevice,String>> = taskrunner.collect();

    if !results.is_empty()  {
        info!("Listing found devices");
        info!("#####################");
        for device in results.into_iter().flatten() {
            info!("{}", device);
        }
    }

    Ok(())
}
