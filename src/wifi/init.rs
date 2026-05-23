use anyhow::Result;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, Configuration, EspWifi};
pub fn start_ap() -> Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs))?;
    let ap_config = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "ESP32-SETUP".try_into().unwrap(),
        password: "12345678".try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        channel: 1,
        ssid_hidden: false,
        max_connections: 4,
        ..Default::default()
    });
    wifi.set_configuration(&ap_config)?;
    wifi.start()?;
    core::mem::forget(wifi);
    Ok(())
}
