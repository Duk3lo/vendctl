use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};

pub static BOT_RESTART_SIGNAL: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomCommand {
    pub trigger: String,
    pub response: String,
    #[serde(default)]
    pub is_app_cmd: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub token: String,
    pub app_id: String,
    pub custom_commands: Vec<CustomCommand>,
    pub slash_commands: Vec<CustomCommand>,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false, token: "".to_string(), app_id: "".to_string(),
            custom_commands: vec![ CustomCommand { trigger: "!ping".to_string(), response: "Pong!".to_string(), is_app_cmd: false } ],
            slash_commands: vec![ CustomCommand { trigger: "hola".to_string(), response: "Hola desde Slash!".to_string(), is_app_cmd: true } ],
        }
    }
}

pub fn get_config(nvs_partition: &EspDefaultNvsPartition) -> Result<DiscordConfig> {
    let nvs = EspNvs::new(nvs_partition.clone(), "discord_data", true)?;
    let mut buffer = vec![0u8; 3072]; 
    match nvs.get_str("config", &mut buffer)? {
        Some(json_str) => Ok(serde_json::from_str(json_str).unwrap_or_default()),
        _ => Ok(DiscordConfig::default()),
    }
}

pub fn save_config(nvs_partition: &EspDefaultNvsPartition, config: &DiscordConfig) -> Result<()> {
    let json_str = serde_json::to_string(config)?;
    let nvs = EspNvs::new(nvs_partition.clone(), "discord_data", true)?;
    nvs.set_str("config", &json_str)?;
    
    BOT_RESTART_SIGNAL.store(true, Ordering::Relaxed);
    Ok(())
}