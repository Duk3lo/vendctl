use crate::web::auth;
use crate::wifi::{init::SharedWifi, scanner, connection};
use anyhow::Result;
use esp_idf_svc::http::server::{Configuration, EspHttpServer, Request};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use serde::Deserialize;
use crate::wifi::storage::{self, SavedNetwork};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use crate::system;

const LOGIN_HTML: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/login.html.gz"));
const DASHBOARD_HTML: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dashboard.html.gz"));
const STYLE_CSS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/style.css.gz"));
const SCRIPT_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/script.js.gz"));

#[derive(Deserialize)]
struct ApToggleReq { enable: bool }

#[derive(Deserialize)]
struct ForgetReq { ssid: String }

fn get_session_token(req: &Request<&mut esp_idf_svc::http::server::EspHttpConnection>) -> Option<String> {
    req.header("Cookie").and_then(|c| {
        c.split(';').find_map(|s| {
            let p: Vec<&str> = s.trim().split('=').collect();
            if p.len() == 2 && p[0] == "session_id" { Some(p[1].to_string()) } else { None }
        })
    })
}

pub fn start_web(wifi: SharedWifi, nvs: EspDefaultNvsPartition) -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/", Method::Get, |req| -> Result<()> {
        req.into_response(302, Some("Found"), &[("Location", "/login")])?.write_all(b"")?;
        Ok(())
    })?;

    server.fn_handler("/login", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "text/html")])?.write_all(LOGIN_HTML)?;
        Ok(())
    })?;

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

    server.fn_handler("/dashboard", Method::Get, |req| -> Result<()> {
        let t = get_session_token(&req);
        if t.is_none() || !auth::is_valid_session(&t.unwrap()) {
            return req.into_response(302, Some("Found"), &[("Location", "/login")])?.write_all(b"").map_err(|e| e.into());
        }
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "text/html")])?.write_all(DASHBOARD_HTML)?;
        Ok(())
    })?;

    server.fn_handler("/api/wifi/scan", Method::Get, {
        let w = wifi.clone();
        move |req| -> Result<()> {
            let networks = scanner::scan_networks(w.clone())?;
            let json = serde_json::to_string(&networks)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?.write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/connect", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            let mut b = [0u8; 1024];
            let l = req.read(&mut b)?;
            let d: SavedNetwork = serde_json::from_slice(&b[..l])?;
            let conn_req = connection::ConnectRequest {
                ssid: d.ssid.clone(), pass: d.pass.clone(), auth_type: d.auth_type.clone(),
                user: d.user.clone(), anon_identity: d.anon_identity.clone(), eap_method: d.eap_method.clone(), phase2: d.phase2.clone(),
            };
            storage::save_network(&nvs_clone, d)?;
            connection::connect_to_wifi(w.clone(), &nvs_clone, conn_req)?;
            req.into_response(200, Some("OK"), &[])?.write_all(b"Connecting and saved...")?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/saved", Method::Get, {
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            let nets = storage::get_saved_networks(&nvs_clone).unwrap_or_default();
            let json = serde_json::to_string(&nets)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?.write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/forget", Method::Post, {
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            let mut b = [0u8; 128];
            let l = req.read(&mut b)?;
            let d: ForgetReq = serde_json::from_slice(&b[..l])?;
            storage::delete_network(&nvs_clone, &d.ssid)?;
            req.into_response(200, Some("OK"), &[])?.write_all(b"Deleted")?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-status", Method::Get, {
        let w = wifi.clone();
        move |req| -> Result<()> {
            let status = connection::get_ap_status(w.clone()).unwrap_or(false);
            let json = format!("{{\"enabled\": {}}}", status);
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?.write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-toggle", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            let t = get_session_token(&req);
            if t.is_none() || !auth::is_valid_session(&t.unwrap()) {
                return req.into_response(401, Some("Unauthorized"), &[])?.write_all(b"401").map_err(|e| e.into());
            }
            let mut b = [0u8; 128];
            let l = req.read(&mut b)?;
            let d: ApToggleReq = serde_json::from_slice(&b[..l])?;
            match connection::set_ap_status(w.clone(), &nvs_clone, d.enable) {
                Ok(_) => req.into_response(200, Some("OK"), &[])?.write_all(b"OK")?,
                Err(_) => req.into_response(500, Some("Error"), &[])?.write_all(b"Error")?,
            };
            Ok(())
        }
    })?;

    server.fn_handler("/api/wifi/ap-config", Method::Get, {
        let nvs_clone = nvs.clone();
        move |req| -> Result<()> {
            let config = storage::get_ap_config(&nvs_clone).unwrap_or_default();
            let json = serde_json::to_string(&config)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?.write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    // GUARDAR CONFIG DEL AP
    server.fn_handler("/api/wifi/ap-config", Method::Post, {
        let w = wifi.clone();
        let nvs_clone = nvs.clone();
        move |mut req| -> Result<()> {
            let t = get_session_token(&req);
            if t.is_none() || !auth::is_valid_session(&t.unwrap()) {
                return req.into_response(401, Some("Unauthorized"), &[])?.write_all(b"401").map_err(|e| e.into());
            }
            let mut b = [0u8; 256];
            let l = req.read(&mut b)?;
            let d: storage::ApConfig = serde_json::from_slice(&b[..l])?;
            match connection::update_ap_config(w.clone(), &nvs_clone, d) {
                Ok(_) => req.into_response(200, Some("OK"), &[])?.write_all(b"OK")?,
                Err(_) => req.into_response(500, Some("Error"), &[])?.write_all(b"Error")?,
            };
            Ok(())
        }
    })?;

    server.fn_handler("/style.css", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "text/css")])?.write_all(STYLE_CSS)?;
        Ok(())
    })?;

    server.fn_handler("/script.js", Method::Get, |req| -> Result<()> {
        req.into_response(200, Some("OK"), &[("Content-Encoding", "gzip"), ("Content-Type", "application/javascript")])?.write_all(SCRIPT_JS)?;
        Ok(())
    })?;

    server.fn_handler("/api/logout", Method::Post, |req| -> Result<()> {
        if let Some(token) = get_session_token(&req) { auth::logout(&token); }
        let c = "session_id=; HttpOnly; Path=/; Max-Age=0";
        req.into_response(302, Some("Found"), &[("Set-Cookie", c), ("Location", "/login")])?.write_all(b"")?;
        Ok(())
    })?;

    server.fn_handler("/api/system/status", Method::Get, {
        let w = wifi.clone();
        move |req| -> Result<()> {
            let status = system::get_status(w.clone());
            let json = serde_json::to_string(&status)?;
            req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?.write_all(json.as_bytes())?;
            Ok(())
        }
    })?;

    Ok(server)
}