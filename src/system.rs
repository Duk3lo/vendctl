use serde::Serialize;
use crate::wifi::init::SharedWifi;
use esp_idf_svc::wifi::Configuration;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use crate::discord::storage as discord_storage;
use esp_idf_svc::sys::{
    heap_caps_get_total_size, heap_caps_get_free_size, MALLOC_CAP_8BIT,
    nvs_get_stats, nvs_stats_t,
    esp_wifi_sta_get_ap_info, wifi_ap_record_t
};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

lazy_static::lazy_static! {
    static ref INTERNET_STATUS: Mutex<(bool, Option<Instant>)> = Mutex::new((false, None));
    pub static ref DISCORD_PING_MS: AtomicU32 = AtomicU32::new(0);
    pub static ref DISCORD_IS_RUNNING: AtomicBool = AtomicBool::new(false);
}

pub fn check_internet_cached() -> bool {
    let mut status = INTERNET_STATUS.lock().unwrap();
    let now = Instant::now();
    if let Some(last_check) = status.1 {
        if now.duration_since(last_check).as_secs() < 10 {
            return status.0;
        }
    }
    let has_net = match TcpStream::connect_timeout(&"8.8.8.8:53".parse().unwrap(), Duration::from_secs(1)) {
        Ok(_) => true,
        Err(_) => false,
    };
    *status = (has_net, Some(now));
    has_net
}

#[derive(Serialize)]
pub struct SystemStatus {
    pub wifi_connected: bool,
    pub has_internet: bool,
    pub discord_enabled: bool,
    pub discord_running: bool,
    pub discord_ping: u32,
    pub wifi_ssid: String,
    pub wifi_rssi: i8,
    pub ap_enabled: bool,
    pub ap_ssid: String,
    pub ram_total: u32,
    pub ram_free: u32,
    pub nvs_total: u32,
    pub nvs_used: u32,
}

pub fn get_status(wifi: SharedWifi, nvs: &EspDefaultNvsPartition) -> SystemStatus {
    let ram_total = unsafe { heap_caps_get_total_size(MALLOC_CAP_8BIT as u32) as u32 };
    let ram_free = unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT as u32) as u32 };

    let mut nvs_stats: nvs_stats_t = Default::default();
    unsafe { nvs_get_stats(b"nvs\0".as_ptr() as *const _, &mut nvs_stats); }

    let discord_enabled = discord_storage::get_config(nvs).map(|c| c.enabled).unwrap_or(false);

    let mut wifi_connected = false;
    let mut has_internet = false;
    let mut wifi_ssid = String::from("Desconectado");
    let mut wifi_rssi = 0;
    let mut ap_enabled = false;
    let mut ap_ssid = String::from("Desactivada");

    if let Ok(w) = wifi.lock() {
        wifi_connected = w.is_connected().unwrap_or(false);
        if let Ok(config) = w.get_configuration() {
            match config {
                Configuration::Client(c) => { if wifi_connected { wifi_ssid = c.ssid.to_string(); } },
                Configuration::Mixed(c, ap) => {
                    if wifi_connected { wifi_ssid = c.ssid.to_string(); }
                    ap_enabled = true;
                    ap_ssid = ap.ssid.to_string();
                },
                Configuration::AccessPoint(ap) => { ap_enabled = true; ap_ssid = ap.ssid.to_string(); }
                _ => {}
            }
        }
        if wifi_connected {
            let mut ap_info: wifi_ap_record_t = Default::default();
            unsafe { if esp_wifi_sta_get_ap_info(&mut ap_info) == 0 { wifi_rssi = ap_info.rssi; } }
            has_internet = check_internet_cached();
        }
    }

    SystemStatus {
        wifi_connected, has_internet, discord_enabled,
        discord_running: DISCORD_IS_RUNNING.load(Ordering::Relaxed),
        discord_ping: DISCORD_PING_MS.load(Ordering::Relaxed),
        wifi_ssid, wifi_rssi, ap_enabled, ap_ssid, ram_total, ram_free,
        nvs_total: nvs_stats.total_entries as u32,
        nvs_used: nvs_stats.used_entries as u32,
    }
}

pub fn format_placeholders(text: &str) -> String {
    // ESTA LÍNEA ES LA MAGIA: Convierte el "\n" literal en un salto de línea real para Discord
    let mut result = text.replace("\\n", "\n");

    let ram_total = unsafe { (heap_caps_get_total_size(MALLOC_CAP_8BIT as u32) / 1024) as u32 };
    let ram_free = unsafe { (esp_idf_svc::sys::esp_get_free_heap_size() / 1024) as u32 };
    let ram_used = ram_total - ram_free;
    let ram_min = unsafe { esp_idf_svc::sys::esp_get_minimum_free_heap_size() / 1024 };
    
    // AQUÍ QUITAMOS LOS "KB" PARA QUE NO SALGAN DOBLE
    result = result.replace("{{RAM_TOTAL}}", &format!("{}", ram_total));
    result = result.replace("{{RAM_FREE}}", &format!("{}", ram_free));
    result = result.replace("{{RAM_USED}}", &format!("{}", ram_used));
    result = result.replace("{{RAM_MIN}}", &format!("{}", ram_min));

    let mut nvs_stats: nvs_stats_t = Default::default();
    unsafe { nvs_get_stats(b"nvs\0".as_ptr() as *const _, &mut nvs_stats); }
    let nvs_total = nvs_stats.total_entries as u32;
    let nvs_used = nvs_stats.used_entries as u32;
    let nvs_free = nvs_total - nvs_used;
    result = result.replace("{{NVS_TOTAL}}", &format!("{}", nvs_total));
    result = result.replace("{{NVS_USED}}", &format!("{}", nvs_used));
    result = result.replace("{{NVS_FREE}}", &format!("{}", nvs_free));

    // AQUÍ QUITAMOS EL "ms"
    let ping = DISCORD_PING_MS.load(Ordering::Relaxed);
    result = result.replace("{{PING}}", &format!("{}", ping));
    
    let mut ap_info: esp_idf_svc::sys::wifi_ap_record_t = Default::default();
    let (rssi, ssid) = unsafe {
        if esp_idf_svc::sys::esp_wifi_sta_get_ap_info(&mut ap_info) == 0 {
            let s = std::ffi::CStr::from_ptr(ap_info.ssid.as_ptr() as *const _).to_string_lossy().into_owned();
            (ap_info.rssi, s)
        } else {
            (0, "Desconectado".to_string())
        }
    };
    
    // AQUÍ QUITAMOS EL "dBm"
    result = result.replace("{{RSSI}}", &format!("{}", rssi));
    result = result.replace("{{SSID}}", &ssid);
    
    let total_secs = unsafe { esp_idf_svc::sys::esp_timer_get_time() / 1_000_000 };
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    result = result.replace("{{UPTIME}}", &format!("{:02}:{:02}:{:02}", hours, mins, secs));

    result
}