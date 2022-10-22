//! structs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Used for matching the friendly name command
/// ```
/// use serde_json;
/// use tasmota::FriendlyName1;
///
/// let input = "{ \"FriendlyName1\": \"tasmota_DC7194\" }";
/// let parsed: FriendlyName1 = serde_json::from_str(&input).unwrap();
/// ```
///
pub struct FriendlyName1 {
    #[serde(rename = "FriendlyName1")]
    /// like it says on the tin
    pub friendly_name_1: String,
}

/// Used for matching the device name command
/// ```
/// use serde_json;
/// use tasmota::DeviceName;
///
/// let input = "{ \"DeviceName\": \"tasmota_DC7194\" }";
/// let parsed: DeviceName = serde_json::from_str(&input).unwrap();
/// ```
///
#[derive(Debug, Serialize, Deserialize)]
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
            "{{ip={:?}, FriendlyName1={:?}, DeviceName={:?}, mac_address={:?}, version={:?} }}",
            self.ip,
            self.friendly_name_1
                .as_ref()
                .unwrap_or(&String::from("blank")),
            self.device_name.as_ref().unwrap_or(&String::from("blank")),
            self.mac_address.as_ref().unwrap_or(&String::from("blank")),
            self.version.as_ref().unwrap_or(&String::from("blank")),
        )
    }
}

/// Represents configuration of the app
#[derive(Debug, Clone)]
pub struct TasmotaConfig {
    /// username for access
    pub username: String,
    /// password for access
    pub password: Option<String>,
}
