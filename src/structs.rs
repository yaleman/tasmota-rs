// structs
// use std::net::Ipv4Addr;


use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendlyName1 {
    #[serde(rename = "FriendlyName1")]
    pub friendly_name_1: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceName {
    #[serde(rename = "DeviceName")]
    pub device_name: String,
}

#[derive(Clone, Debug)]
pub struct TasmotaDevice {
    pub ip: std::net::IpAddr,
    pub username: String,
    pub password: Option<String>,
    pub friendly_name_1: Option<String>,
}

impl TasmotaDevice {

    pub fn new(ip: std::net::IpAddr,
            username: String,
            password: Option<String>
        ) -> Self {
        TasmotaDevice{
            ip,
            username: username.to_string(),
            password: password.to_owned(),
            friendly_name_1: None,
        }
    }
}
