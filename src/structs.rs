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

#[derive(Clone, Copy)]
pub struct TasmotaDevice {
    ip: std::net::IpAddr,
    username: &'static str,
    password: Option<&'static str>,

}

impl TasmotaDevice {

    pub fn new(ip: std::net::IpAddr,
            username: &'static str,
            password: Option<&'static str>
        ) -> Self {
        TasmotaDevice{
            ip, username, password
        }
    }

    fn get_baseurl(self) -> String {
        format!("http://{}", self.ip)
    }

    pub fn get_cmnd(self, client: &reqwest::blocking::Client, cmnd: &str) ->
        Result<reqwest::blocking::Response, ()> {
    match client.get(format!("{}/cm?cmnd={}", &self.get_baseurl(), cmnd))
        .basic_auth(
            &self.username,
            self.password)
        .send() {
            Ok(value) => Ok(value),
            Err(error) => panic!("Error getting {}/cm?cmnd={}: {:?}", self.get_baseurl(), cmnd, error)
        }
}
}