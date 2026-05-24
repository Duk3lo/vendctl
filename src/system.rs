use serde::Serialize;
use crate::wifi::init::SharedWifi;
use esp_idf_svc::wifi::Configuration;
use esp_idf_svc::sys::{
    heap_caps_get_total_size, heap_caps_get_free_size, MALLOC_CAP_8BIT,
    nvs_get_stats, nvs_stats_t,
    esp_wifi_sta_get_ap_info, wifi_ap_record_t
};

#[derive(Serialize)]
pub struct SystemStatus {
    pub wifi_connected: bool,
    pub wifi_ssid: String,
    pub wifi_rssi: i8,
    pub ap_enabled: bool,
    pub ap_ssid: String,
    pub ram_total: u32,
    pub ram_free: u32,
    pub nvs_total: u32,
    pub nvs_used: u32,
    pub ws_status: String,
}

pub fn get_status(wifi: SharedWifi) -> SystemStatus {
    let ram_total = unsafe { heap_caps_get_total_size(MALLOC_CAP_8BIT as u32) as u32 };
    let ram_free = unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT as u32) as u32 };

    let mut nvs_stats: nvs_stats_t = Default::default();
    unsafe {
        nvs_get_stats(b"nvs\0".as_ptr() as *const _, &mut nvs_stats);
    }

    let mut wifi_connected = false;
    let mut wifi_ssid = String::from("Desconectado");
    let mut wifi_rssi = 0;
    let mut ap_enabled = false;
    let mut ap_ssid = String::from("Desactivada");

    if let Ok(w) = wifi.lock() {
        wifi_connected = w.is_connected().unwrap_or(false);
        
        if let Ok(config) = w.get_configuration() {
            match config {
                Configuration::Client(c) => {
                    if wifi_connected { wifi_ssid = c.ssid.to_string(); }
                },
                Configuration::Mixed(c, ap) => {
                    if wifi_connected { wifi_ssid = c.ssid.to_string(); }
                    ap_enabled = true;
                    ap_ssid = ap.ssid.to_string();
                },
                Configuration::AccessPoint(ap) => {
                    ap_enabled = true;
                    ap_ssid = ap.ssid.to_string();
                }
                _ => {}
            }
        }
            
        if wifi_connected {
            let mut ap_info: wifi_ap_record_t = Default::default();
            unsafe {
                if esp_wifi_sta_get_ap_info(&mut ap_info) == 0 {
                    wifi_rssi = ap_info.rssi;
                }
            }
        }
    }

    SystemStatus {
        wifi_connected,
        wifi_ssid,
        wifi_rssi,
        ap_enabled,
        ap_ssid,
        ram_total,
        ram_free,
        nvs_total: nvs_stats.total_entries as u32,
        nvs_used: nvs_stats.used_entries as u32,
        ws_status: "Desconectado".to_string(), 
    }
}