mod structs;

use crate::structs::*;
use ipnet::Ipv4Net;

#[derive(Debug, Clone)]
pub struct TasmotaConfig {
    pub username: String,
    pub password: String,
}

fn main() {
    let config_file = String::from("~/.config/tasmota-rs.json");
    let config_filename: String = shellexpand::tilde(&config_file).into_owned();
    let mut config = config::Config::default();
    config
        .merge(config::File::with_name(&config_filename))
        .unwrap();


    let username: &'static str = config.get("username").unwrap();
    let password: Option<&'static str> = config.get("password").unwrap();

    let net: Ipv4Net = config.get("ip_range").unwrap_or("10.0.0.0/24").parse().unwrap();
    // let host = String::from("http://10.0.5.129");
    // println!("!");
    let client = reqwest::blocking::Client::new();

    for ip in net.hosts() {
        let device = crate::structs::TasmotaDevice::new(
            ip.into(),
            username,
            password
        );

        let friendly_name_1 = &device.get_cmnd(&client, "FriendlyName1").unwrap();
        let devicename = &device.get_cmnd(&client, "DeviceName").unwrap();

    let friendly_name_1: FriendlyName1 = match friendly_name_1.json() {
        Ok(value) => value,
        Err(error) => panic!("Failed to get FriendlyName1: {:?}", error)
    };

    let devicename: DeviceName = match devicename.json() {
        Ok(value) => value,
        Err(error) => panic!("Failed to get DeviceName: {:?}", error)
    };

    eprintln!("devicename: \t{:?}", devicename.device_name);
    eprintln!("friendly_name_1: \t{:?}", friendly_name_1.friendly_name_1);

    }



}
