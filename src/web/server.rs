use crate::discord::storage::{self as discord_storage, DiscordConfig};
use crate::system;
use crate::web::auth;
use crate::wifi::storage::{self, SavedNetwork};
use crate::wifi::{connection, init::SharedWifi, scanner};
use anyhow::Result;
use esp_idf_svc::http::server::{Configuration, EspHttpServer, Request};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use serde::Deserialize;

// =========================================================================
// CARGA DE ARCHIVOS COMPRIMIDOS (GZIP) DESDE OUT_DIR
// =========================================================================
const LOGIN_HTML: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/login.html.gz"));
const DASHBOARD_HTML: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dashboard.html.gz"));

const STYLE_CSS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/css/style.css.gz"));

const JS_AUTH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js/auth.js.gz"));
const JS_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js/ui.js.gz"));
const JS_WIFI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js/wifi.js.gz"));
const JS_DISCORD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js/discord.js.gz"));
const JS_SYSTEM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js/system.js.gz"));
// =========================================================================

#[derive(Deserialize)]
struct ApToggleReq {
    enable: bool,
}

#[derive(Deserialize)]
struct ForgetReq {
    ssid: String,
}

fn get_session_token(
    req: &Request<&mut esp_idf_svc::http::server::EspHttpConnection>,
) -> Option<String> {
    req.header("Cookie").and_then(|c| {
        c.split(';').find_map(|s| {
            let p: Vec<&str> = s.trim().split('=').collect();
            if p.len() == 2 && p[0] == "session_id" {
                Some(p[1].to_string())
            } else {
                None
            }
        })
    })
}

fn is_authorized(req: &Request<&mut esp_idf_svc::http::server::EspHttpConnection>) -> bool {
    if let Some(token) = get_session_token(req) {
        return auth::is_valid_session(&token);
    }
    false
}

pub fn start_web(wifi: SharedWifi, nvs: EspDefaultNvsPartition) -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&Configuration::default())?;

    // --- RUTAS DE REDIRECCIÓN Y HTML ---
    server.fn_handler("/", Method::Get, |req| -> Result<()> {
        req.into_response(302, Some("Found"), &[("Location", "/login")])?
            .write_all(b"")?;
        Ok(())
    })?;

    server.fn_handler("/login", Method::Get, |req| -> Result<()> {
        req.into_response(
            200,
            Some("OK"),
            &[("Content-Encoding", "gzip"), ("Content-Type", "text/html")],
        )?
        .write_all(LOGIN_HTML)?;
        Ok(())
    })?;

    server.fn_handler("/dashboard", Method::Get, |req| -> Result<()> {
        if !is_authorized(&req) {
            return req
                .into_response(302, Some("Found"), &[("Location", "/login")])?
                .write_all(b"")
                .map_err(|e| e.into());
        }
        req.into_response(
            200,
            Some("OK"),
            &[("Content-Encoding", "gzip"), ("Content-Type", "text/html")],
        )?
        .write_all(DASHBOARD_HTML)?;
        Ok(())
    })?;

    // --- RUTAS DE ARCHIVOS ESTÁTICOS (CSS y JS) ---
    server.fn_handler("/css/style.css", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "text/css")])?.write_all(STYLE_CSS)?;
        Ok(())
    })?;

    server.fn_handler("/js/auth.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(JS_AUTH)?;
        Ok(())
    })?;

    server.fn_handler("/js/ui.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(JS_UI)?;
        Ok(())
    })?;

    server.fn_handler("/js/wifi.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(JS_WIFI)?;
        Ok(())
    })?;

    server.fn_handler("/js/discord.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(JS_DISCORD)?;
        Ok(())
    })?;

    server.fn_handler("/js/system.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(JS_SYSTEM)?;
        Ok(())
    })?;


    // --- RUTAS API (BACKEND) ---
    server.fn_handler("/api/login", Method::Post, |mut req| -> Result<()> {
        let mut b = [0u8; 512];
        let l = req.read(&mut b).unwrap_or(0);
        let body_str = String::from_utf8_lossy(&b[..l]);

        if body_str.contains("admin") && body_str.contains("12345") {
            let t = auth::login();
            let c = format!("session_id={}; HttpOnly; Path=/", t);
            let success_html = b"<meta http-equiv='refresh' content='0; url=/dashboard' /><script>window.location.href='/dashboard';</script>";
            req.into_response(200, Some("OK"), &[("Set-Cookie", c.as_str()), ("Content-Type", "text/html")])?.write_all(success_html)?;
        } else {
            req.into_response(401, Some("Unauthorized"), &[("Content-Type", "text/html")])?.write_all(b"<script>alert('Credenciales incorrectas'); window.location.href='/login';</script>")?;
        }
        Ok(())
    })?;

    server.fn_handler("/api/system/status", Method::Get, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let status = system::get_status(w.clone(), &nvs_clone);
            let json = serde_json::to_string(&status)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/scan", Method::Get, {
        let w = wifi.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let networks = scanner::scan_networks(w.clone())?;
            let json = serde_json::to_string(&networks)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/connect", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let mut b = [0u8; 1024];
            let l = req.read(&mut b)?;
            let d: SavedNetwork = serde_json::from_slice(&b[..l])?;
            let conn_req = connection::ConnectRequest {
                ssid: d.ssid.clone(),
                pass: d.pass.clone(),
                auth_type: d.auth_type.clone(),
                user: d.user.clone(),
                anon_identity: d.anon_identity.clone(),
                eap_method: d.eap_method.clone(),
                phase2: d.phase2.clone(),
            };
            storage::save_network(&nvs_clone, d)?;
            connection::connect_to_wifi(w.clone(), &nvs_clone, conn_req)?;
            req.into_response(200, Some("OK"), &[])?
                .write_all(b"Connecting and saved...")?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/saved", Method::Get, {
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let nets = storage::get_saved_networks(&nvs_clone).unwrap_or_default();
            let json = serde_json::to_string(&nets)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/forget", Method::Post, {
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let mut b = [0u8; 128];
            let l = req.read(&mut b)?;
            let d: ForgetReq = serde_json::from_slice(&b[..l])?;
            storage::delete_network(&nvs_clone, &d.ssid)?;
            req.into_response(200, Some("OK"), &[])?
                .write_all(b"Deleted")?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-status", Method::Get, {
        let w = wifi.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let status = connection::get_ap_status(w.clone()).unwrap_or(false);
            let json = format!("{{\"enabled\": {}}}", status);
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-toggle", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let mut b = [0u8; 128];
            let l = req.read(&mut b)?;
            let d: ApToggleReq = serde_json::from_slice(&b[..l])?;
            match connection::set_ap_status(w.clone(), &nvs_clone, d.enable) {
                Ok(_) => req.into_response(200, Some("OK"), &[])?.write_all(b"OK")?,
                Err(_) => req
                    .into_response(500, Some("Error"), &[])?
                    .write_all(b"Error")?,
            };
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-config", Method::Get, {
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let config = storage::get_ap_config(&nvs_clone).unwrap_or_default();
            let json = serde_json::to_string(&config)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-config", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"Sesion expirada")?;
                return Ok(());
            }
            let mut b = [0u8; 256];
            let l = req.read(&mut b)?;
            let d: storage::ApConfig = serde_json::from_slice(&b[..l])?;
            match connection::update_ap_config(w.clone(), &nvs_clone, d) {
                Ok(_) => req.into_response(200, Some("OK"), &[])?.write_all(b"OK")?,
                Err(_) => req
                    .into_response(500, Some("Error"), &[])?
                    .write_all(b"Error")?,
            };
            Ok(())
        }
    })?;

    server.fn_handler("/api/discord/config", Method::Get, {
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"")?;
                return Ok(());
            }
            let config = discord_storage::get_config(&nvs_clone).unwrap_or_default();
            let json = serde_json::to_string(&config)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
                .write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/discord/config", Method::Post, {
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            if !is_authorized(&req) {
                req.into_response(401, Some("Unauthorized"), &[])?
                    .write_all(b"")?;
                return Ok(());
            }
            // ¡MUY IMPORTANTE! Vec! de 5KB para soportar Embeds grandes sin Crash.
            let mut b = vec![0u8; 5120];
            let l = req.read(&mut b)?;
            let d: DiscordConfig = match serde_json::from_slice(&b[..l]) {
                Ok(config) => config,
                Err(e) => {
                    println!("❌ Error de parseo: {:?}", e);
                    req.into_response(400, Some("Bad Request"), &[])?
                        .write_all(b"JSON Invalido o muy largo")?;
                    return Ok(());
                }
            };

            discord_storage::save_config(&nvs_clone, &d)?;
            req.into_response(200, Some("OK"), &[])?
                .write_all(b"Guardado")?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/logout", Method::Post, |req| -> Result<()> {
        if let Some(token) = get_session_token(&req) {
            auth::logout(&token);
        }
        let c = "session_id=; HttpOnly; Path=/; Max-Age=0";
        req.into_response(
            302,
            Some("Found"),
            &[("Set-Cookie", c), ("Location", "/login")],
        )?
        .write_all(b"")?;
        Ok(())
    })?;

    Ok(server)
}