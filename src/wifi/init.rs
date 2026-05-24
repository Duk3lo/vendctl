use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, Configuration, EspWifi, ClientConfiguration};
use std::sync::{Arc, Mutex};
use crate::wifi::storage;

pub type SharedWifi = Arc<Mutex<EspWifi<'static>>>;

pub fn start_wifi(nvs: EspDefaultNvsPartition) -> Result<SharedWifi> {
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs.clone()))?;
    
    let ap_saved = storage::get_ap_config(&nvs).unwrap_or_default();
    
    let mut ap_config = AccessPointConfiguration::default();
    ap_config.ssid = ap_saved.ssid.as_str().try_into().unwrap_or_default();
    ap_config.max_connections = 4;
    
    if ap_saved.open {
        ap_config.auth_method = AuthMethod::None;
    } else {
        ap_config.password = ap_saved.pass.as_str().try_into().unwrap_or_default();
        ap_config.auth_method = AuthMethod::WPA2Personal;
    }

    let config = Configuration::Mixed(ClientConfiguration::default(), ap_config);
    
    wifi.set_configuration(&config)?;
    wifi.start()?;
    Ok(Arc::new(Mutex::new(wifi)))
}