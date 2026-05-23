use anyhow::Result;
use esp_idf_svc::http::server::{Configuration, EspHttpServer, Method};

static INDEX_HTML: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/index.html.gz"));

static STYLE_CSS: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/style.css.gz"));

static SCRIPT_JS: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/script.js.gz"));

pub fn start_web() -> Result<()> {
    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/", Method::Get, |request| {
        let mut response = request.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "text/html"),
                ("Content-Encoding", "gzip"),
            ],
        )?;

        response.write(INDEX_HTML)?;

        Ok::<(), anyhow::Error>(())
    })?;

    server.fn_handler("/style.css", Method::Get, |request| {
        let mut response = request.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "text/css"),
                ("Content-Encoding", "gzip"),
            ],
        )?;

        response.write(STYLE_CSS)?;

        Ok::<(), anyhow::Error>(())
    })?;

    server.fn_handler("/script.js", Method::Get, |request| {
        let mut response = request.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "application/javascript"),
                ("Content-Encoding", "gzip"),
            ],
        )?;

        response.write(SCRIPT_JS)?;

        Ok::<(), anyhow::Error>(())
    })?;

    server.fn_handler("/login", Method::Post, |mut request| {
        let mut body = [0_u8; 512];

        let size = request.read(&mut body)?;
        let data = std::str::from_utf8(&body[..size]).unwrap_or("");

        let ok = data.contains("\"user\":\"admin\"")
            && data.contains("\"pass\":\"1234\"");

        let mut response = request.into_ok_response()?;

        if ok {
            response.write(b"LOGIN OK")?;
        } else {
            response.write(b"LOGIN ERROR")?;
        }

        Ok::<(), anyhow::Error>(())
    })?;

    core::mem::forget(server);

    Ok(())
}