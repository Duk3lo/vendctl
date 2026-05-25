use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ws::client::{EspWebSocketClient, EspWebSocketClientConfig, WebSocketEventType};
use esp_idf_svc::ws::FrameType;

use serde_json::{json, Value};

use crate::discord::storage::{get_config, DiscordConfig, BOT_RESTART_SIGNAL};
use crate::system::{check_internet_cached, format_placeholders, DISCORD_PING_MS, DISCORD_IS_RUNNING};

const WS_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const API_URL: &str = "https://discord.com/api/v10";

enum BotEvent {
    Op10Hello(u64),
    Op11HeartbeatAck,
    Message(String, String),
    Interaction(String, String, String),
}

pub fn start_bot_thread(nvs: EspDefaultNvsPartition) {
    std::thread::Builder::new()
        .name("discord_bot".to_string())
        .stack_size(10240)
        .spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            loop {
                if BOT_RESTART_SIGNAL.load(Ordering::Relaxed) {
                    BOT_RESTART_SIGNAL.store(false, Ordering::Relaxed);
                }
                let config = get_config(&nvs).unwrap_or_default();
                if config.enabled && !config.token.is_empty() {
                    if !check_internet_cached() {
                        DISCORD_IS_RUNNING.store(false, Ordering::Relaxed);
                        DISCORD_PING_MS.store(0, Ordering::Relaxed);
                        std::thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                    DISCORD_IS_RUNNING.store(true, Ordering::Relaxed);
                    println!("🌐 Internet detectado. Iniciando servicios de Discord...");
                    register_slash_commands(&config);
                    run_websocket(&config);
                    DISCORD_IS_RUNNING.store(false, Ordering::Relaxed);
                } else {
                    DISCORD_IS_RUNNING.store(false, Ordering::Relaxed);
                    DISCORD_PING_MS.store(0, Ordering::Relaxed);
                }

                std::thread::sleep(Duration::from_secs(5));
            }
        })
        .unwrap();
}

fn run_websocket(config: &DiscordConfig) {
    let connection_config = EspWebSocketClientConfig {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        buffer_size: 10240,
        task_stack: 8192,
        reconnect_timeout_ms: Duration::from_millis(10000),
        network_timeout_ms: Duration::from_millis(10000),
        ..Default::default()
    };

    let (tx, rx) = mpsc::channel::<BotEvent>();
    let mut json_buffer = String::new();
    let tx_clone = tx.clone();

    let client_result = EspWebSocketClient::new(
        WS_URL,
        &connection_config,
        Duration::from_secs(10),
        move |event_result| match event_result {
            Ok(ws_event) => match ws_event.event_type {
                WebSocketEventType::Text(text) => {
                    json_buffer.push_str(text);
                    if let Ok(v) = serde_json::from_str::<Value>(&json_buffer) {
                        process_discord_event(v, &tx_clone);
                        json_buffer.clear();
                    } else if json_buffer.len() > 16384 {
                        json_buffer.clear();
                    }
                }
                _ => {}
            },
            Err(_) => {}
        },
    );

    let mut client = match client_result {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut hb_interval = 41;
    let mut ticks = 0;
    let mut last_ping = Instant::now();

    loop {
        if BOT_RESTART_SIGNAL.load(Ordering::Relaxed) || !check_internet_cached() {
            break;
        }

        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(BotEvent::Op10Hello(interval)) => {
                hb_interval = (interval / 1000) - 2;
                let identify = json!({
                    "op": 2,
                    "d": {
                        "token": config.token,
                        "intents": 33281,
                        "properties": { "os": "esp32", "browser": "esp-bot", "device": "esp32" }
                    }
                })
                .to_string();
                let _ = client.send(FrameType::Text(false), identify.as_bytes());
            }
            Ok(BotEvent::Op11HeartbeatAck) => {
                DISCORD_PING_MS.store(last_ping.elapsed().as_millis() as u32, Ordering::Relaxed);
            }
            Ok(BotEvent::Message(content, channel_id)) => {
                handle_text_command(&content, &channel_id, config);
            }
            Ok(BotEvent::Interaction(name, id, token)) => {
                handle_slash_command(&name, &id, &token, config);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                ticks += 1;
                if ticks >= hb_interval {
                    let _ = client.send(
                        FrameType::Text(false),
                        json!({"op": 1, "d": null}).to_string().as_bytes(),
                    );
                    last_ping = Instant::now();
                    ticks = 0;
                }
            }
            Err(_) => break,
        }
    }
}

fn process_discord_event(v: Value, tx: &mpsc::Sender<BotEvent>) {
    match v["op"].as_u64() {
        Some(10) => {
            let _ = tx.send(BotEvent::Op10Hello(
                v["d"]["heartbeat_interval"].as_u64().unwrap_or(41250),
            ));
        }
        Some(11) => {
            let _ = tx.send(BotEvent::Op11HeartbeatAck);
        }
        Some(0) => {
            let t = v["t"].as_str().unwrap_or("");
            let d = &v["d"];
            if t == "MESSAGE_CREATE" && d["author"]["bot"].as_bool() != Some(true) {
                let _ = tx.send(BotEvent::Message(
                    d["content"].as_str().unwrap_or("").to_string(),
                    d["channel_id"].as_str().unwrap_or("").to_string(),
                ));
            } else if t == "INTERACTION_CREATE" {
                let _ = tx.send(BotEvent::Interaction(
                    d["data"]["name"].as_str().unwrap_or("").to_string(),
                    d["id"].as_str().unwrap_or("").to_string(),
                    d["token"].as_str().unwrap_or("").to_string(),
                ));
            }
        }
        _ => {}
    }
}

fn handle_text_command(content: &str, channel_id: &str, config: &DiscordConfig) {
    let clean_content = content.trim().to_lowercase();
    for cmd in &config.custom_commands {
        if clean_content == cmd.trigger.trim().to_lowercase() {
            let processed_response = format_placeholders(&cmd.response);
            let url = format!("{}/channels/{}/messages", API_URL, channel_id);
            send_http_req(&url, &json!({ "content": processed_response }).to_string(), &config.token, Method::Post);
            break;
        }
    }
}

fn handle_slash_command(name: &str, id: &str, token: &str, config: &DiscordConfig) {
    for cmd in &config.slash_commands {
        let clean_trigger = cmd.trigger.replace("/", "").replace(" ", "").to_lowercase();
        if name == clean_trigger {
            let processed_response = format_placeholders(&cmd.response);
            let url = format!("{}/interactions/{}/{}/callback", API_URL, id, token);
            let payload = json!({ "type": 4, "data": { "content": processed_response } }).to_string();
            send_http_req(&url, &payload, &config.token, Method::Post);
            break;
        }
    }
}

fn register_slash_commands(config: &DiscordConfig) {
    let url = format!("{}/applications/{}/commands", API_URL, config.app_id);
    let mut commands = Vec::new();
    for cmd in &config.slash_commands {
        let safe_name = cmd.trigger.replace("/", "").replace(" ", "").to_lowercase();
        if safe_name.is_empty() {
            continue;
        }

        let integrations = if cmd.is_app_cmd { vec![0, 1] } else { vec![0] };
        commands.push(json!({
            "name": safe_name,
            "description": "Comando ESP32",
            "type": 1,
            "integration_types": integrations,
            "contexts": [0, 1, 2]
        }));
    }
    send_http_req(
        &url,
        &serde_json::to_string(&commands).unwrap_or_default(),
        &config.token,
        Method::Put,
    );
}

fn send_http_req(url: &str, body: &str, bot_token: &str, method: Method) {
    let mut conf = HttpConfig::default();
    conf.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    if let Ok(conn) = EspHttpConnection::new(&conf) {
        let mut client = Client::wrap(conn);
        let auth = format!("Bot {}", bot_token);
        let content_len = body.len().to_string();

        let headers = [
            ("Authorization", auth.as_str()),
            ("Content-Type", "application/json"),
            ("Content-Length", content_len.as_str()),
            ("Connection", "close"),
        ];

        if let Ok(mut req) = client.request(method, url, &headers) {
            let _ = req.write_all(body.as_bytes());
            if let Ok(mut res) = req.submit() {
                let mut b = [0u8; 64];
                while let Ok(n) = res.read(&mut b) {
                    if n == 0 {
                        break;
                    }
                }
            }
        }
    }
}
