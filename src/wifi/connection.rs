use crate::wifi::init::SharedWifi;
use crate::wifi::storage;
use anyhow::Result;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ConnectRequest {
    pub ssid: String,
    pub pass: String,
    pub auth_type: String,
    pub user: Option<String>,
    pub anon_identity: Option<String>,
    pub eap_method: Option<String>,
    pub phase2: Option<String>,
}

pub fn connect_to_wifi(wifi: SharedWifi, nvs: &EspDefaultNvsPartition, req: ConnectRequest) -> Result<()> {

    let mut wifi_lock = wifi.lock().unwrap();
    let current_config = wifi_lock.get_configuration()?;

    let ap_saved = storage::get_ap_config(nvs).unwrap_or_default();
    let mut fallback_ap = AccessPointConfiguration::default();
    fallback_ap.ssid = ap_saved.ssid.as_str().try_into().unwrap_or_default();
    if ap_saved.open {
        fallback_ap.auth_method = AuthMethod::None;
    } else {
        fallback_ap.password = ap_saved.pass.as_str().try_into().unwrap_or_default();
        fallback_ap.auth_method = AuthMethod::WPA2Personal;
    }

    let ap_config = match current_config {
        Configuration::Mixed(_, ap) | Configuration::AccessPoint(ap) => ap,
        _ => fallback_ap,
    };

    let mut client_config = ClientConfiguration::default();
    client_config.ssid = req.ssid.as_str().try_into().map_err(|_| anyhow::anyhow!("SSID demasiado largo"))?;
    client_config.password = req.pass.as_str().try_into().map_err(|_| anyhow::anyhow!("Contraseña demasiado larga"))?;

    if req.auth_type.contains("Enterprise") {
        client_config.auth_method = AuthMethod::WPA2Enterprise;
    } else if req.pass.is_empty() || req.auth_type == "None" || req.auth_type == "Open" {
        client_config.auth_method = AuthMethod::None;
    } else {
        client_config.auth_method = AuthMethod::WPA2Personal;
    }

    wifi_lock.set_configuration(&Configuration::Mixed(client_config, ap_config))?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    wifi_lock.connect()?;
    
    if let Ok(config) = wifi_lock.get_configuration() {
        if let Configuration::Mixed(client, _) = config {
            let _ = wifi_lock.set_configuration(&Configuration::Client(client));
        }
    }
    
    Ok(())
}

pub fn get_ap_status(wifi: SharedWifi) -> Result<bool> {
    let wifi_lock = wifi.lock().unwrap();
    let config = wifi_lock.get_configuration()?;
    match config {
        Configuration::Mixed(_, _) | Configuration::AccessPoint(_) => Ok(true),
        _ => Ok(false),
    }
}

pub fn set_ap_status(wifi: SharedWifi, nvs: &EspDefaultNvsPartition, enable: bool) -> Result<()> {
    let mut wifi_lock = wifi.lock().unwrap();
    let current_config = wifi_lock.get_configuration()?;
    
    let ap_saved = storage::get_ap_config(nvs).unwrap_or_default();
    let mut ap_config = AccessPointConfiguration::default();
    ap_config.ssid = ap_saved.ssid.as_str().try_into().unwrap_or_default();
    if ap_saved.open {
        ap_config.auth_method = AuthMethod::None;
    } else {
        ap_config.password = ap_saved.pass.as_str().try_into().unwrap_or_default();
        ap_config.auth_method = AuthMethod::WPA2Personal;
    }

    match current_config {
        Configuration::Mixed(client, _) if !enable => {
            wifi_lock.set_configuration(&Configuration::Client(client))?;
        }
        Configuration::Client(client) if enable => {
            wifi_lock.set_configuration(&Configuration::Mixed(client, ap_config))?;
        }
        _ => {}
    }
    Ok(())
}

pub fn update_ap_config(wifi: SharedWifi, nvs: &EspDefaultNvsPartition, config: storage::ApConfig) -> Result<()> {
    storage::save_ap_config(nvs, &config)?;

    let mut wifi_lock = wifi.lock().unwrap();
    let current_config = wifi_lock.get_configuration()?;
    
    let mut new_ap = AccessPointConfiguration::default();
    new_ap.ssid = config.ssid.as_str().try_into().unwrap_or_default();
    if config.open {
        new_ap.auth_method = AuthMethod::None;
    } else {
        new_ap.password = config.pass.as_str().try_into().unwrap_or_default();
        new_ap.auth_method = AuthMethod::WPA2Personal;
    }
    
    match current_config {
        Configuration::Client(client) | Configuration::Mixed(client, _) => {
            wifi_lock.set_configuration(&Configuration::Mixed(client, new_ap))?;
        },
        Configuration::AccessPoint(_) => {
            wifi_lock.set_configuration(&Configuration::AccessPoint(new_ap))?;
        }
        _ => {}
    }
    Ok(())
}