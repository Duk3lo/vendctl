mod wifi;
mod web;
mod system;

use esp_idf_svc::nvs::EspDefaultNvsPartition;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let nvs_partition = EspDefaultNvsPartition::take().expect("Error inicializando NVS");
    let wifi_handle = wifi::init::start_wifi(nvs_partition.clone()).expect("Error iniciando WiFi");
    let _mdns = wifi::mdns::start_mdns().expect("Error iniciando mDNS");
    let _server = web::server::start_web(wifi_handle.clone(), nvs_partition.clone()).expect("Error iniciando Servidor");
    wifi::manager::start_wifi_manager(wifi_handle.clone(), nvs_partition.clone());
    println!("Sistema Iniciado. Servidor web escuchando...");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}