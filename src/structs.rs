//! structs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Used for matching the friendly name command
pub struct FriendlyName1 {
    #[serde(rename = "FriendlyName1")]
    /// like it says on the tin
    pub friendly_name_1: String,
}

#[derive(Debug, Serialize, Deserialize)]
/// Used for matching the device name command
pub struct DeviceName {
    #[serde(rename = "DeviceName")]
    /// like it says on the tin
    pub device_name: String,
}

#[derive(Clone, Debug)]
/// Represents a single device
pub struct TasmotaDevice {
    /// config ip of the device
    pub ip: std::net::IpAddr,
    /// admin username, normally admin
    pub username: String,
    /// password, if needed
    pub password: Option<String>,
    /// version number, if found
    pub version: Option<String>,
    /// friendly name
    pub friendly_name_1: Option<String>,
    /// internal device name
    pub device_name: Option<String>,
    /// wifi mac
    pub mac_address: Option<String>,
}

impl TasmotaDevice {
    /// build a new device from ip/username/password
    pub fn new(ip: std::net::IpAddr, username: String, password: Option<String>) -> Self {
        TasmotaDevice {
            ip,
            username,
            password,
            version: None,
            friendly_name_1: None,
            device_name: None,
            mac_address: None,
        }
    }
}

impl std::fmt::Display for TasmotaDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ip={:?}, FriendlyName1={:?}, DeviceName={:?}, mac_address={:?}}}",
            self.ip,
            self.friendly_name_1
                .as_ref()
                .unwrap_or(&String::from("blank")),
            self.device_name.as_ref().unwrap_or(&String::from("blank")),
            self.mac_address.as_ref().unwrap_or(&String::from("blank")),
        )
    }
}
