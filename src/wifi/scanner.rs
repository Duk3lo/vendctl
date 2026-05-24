use crate::wifi::init::SharedWifi;
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub rssi: i8,
    pub auth_method: String,
}

pub fn scan_networks(wifi: SharedWifi) -> Result<Vec<WifiNetwork>> {
    let mut wifi_lock = wifi.lock().unwrap();
    let ap_infos = wifi_lock.scan()?;
    let mut networks = Vec::new();

    for ap in ap_infos {
        let b = ap.bssid;
        let bssid_str = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        );
        let auth_str = format!("{:?}", ap.auth_method).replace("\"", "");

        networks.push(WifiNetwork {
            ssid: ap.ssid.to_string(),
            bssid: bssid_str,
            rssi: ap.signal_strength,
            auth_method: auth_str,
        });
    }

    Ok(networks)
}