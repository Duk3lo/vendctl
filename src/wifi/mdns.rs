use anyhow::Result;
use esp_idf_svc::mdns::EspMdns;

pub fn start_mdns() -> Result<EspMdns> {
    let mut mdns = EspMdns::take()?;
    mdns.set_hostname("esp32")?;
    mdns.set_instance_name("Panel de Control ESP32")?;
    
    Ok(mdns)
}