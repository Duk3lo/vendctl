use super::init::SharedWifi;
use super::storage::get_saved_networks;
use super::scanner;
use super::connection;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use std::thread;
use std::time::Duration;

pub fn start_wifi_manager(wifi: SharedWifi, nvs_partition: EspDefaultNvsPartition) {
    thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            thread::sleep(Duration::from_secs(5));
            loop {
                let is_connected = wifi.lock().map(|w| w.is_connected().unwrap_or(false)).unwrap_or(false);
                
                if !is_connected {
                    let available = scanner::scan_networks(wifi.clone()).unwrap_or_default();
                    let saved_networks = get_saved_networks(&nvs_partition).unwrap_or_default();
                    
                    let mut se_conecto = false;
                    for saved in saved_networks.into_iter() {
                        if available.iter().any(|n| n.ssid == saved.ssid) {
                            let req = connection::ConnectRequest {
                                ssid: saved.ssid,
                                pass: saved.pass,
                                auth_type: saved.auth_type,
                                user: saved.user,
                                anon_identity: saved.anon_identity,
                                eap_method: saved.eap_method,
                                phase2: saved.phase2,
                            };
                            if connection::connect_to_wifi(wifi.clone(), &nvs_partition, req).is_ok() {
                                se_conecto = true;
                                break;
                            }
                        }
                    }
                    
                    if se_conecto {
                        thread::sleep(Duration::from_secs(15));
                    } else {
                        thread::sleep(Duration::from_secs(10));
                    }
                } else {
                    thread::sleep(Duration::from_secs(30));
                }
            }
        })
        .expect("Error al iniciar el hilo del gestor WiFi");
}