mod wifi;
mod web;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    wifi::init::start_ap().unwrap();
    web::init::start_web().unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
