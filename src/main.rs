use esp_idf_svc::nvs::EspDefaultNvsPartition;
use vendctl::{wifi, web, discord};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let nvs_partition = EspDefaultNvsPartition::take().expect("Error NVS");
    let wifi_handle = wifi::init::start_wifi(nvs_partition.clone()).expect("Error WiFi");
    let _mdns = wifi::mdns::start_mdns().expect("Error mDNS");
    let _server = web::server::start_web(wifi_handle.clone(), nvs_partition.clone()).expect("Error Servidor");
    discord::bot::start_bot_thread(nvs_partition.clone());
    wifi::manager::start_wifi_manager(wifi_handle.clone(), nvs_partition.clone());
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}