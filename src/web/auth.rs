use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

lazy_static::lazy_static! {
    static ref ACTIVE_SESSION: Mutex<Option<(String, Instant)>> = Mutex::new(None);
}

pub fn generate_token() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("esp32_session_{}", timestamp)
}

pub fn login() -> String {
    let token = generate_token();
    let mut session = ACTIVE_SESSION.lock().unwrap();
    *session = Some((token.clone(), Instant::now()));
    
    token
}

pub fn logout(token: &str) {
    let mut session = ACTIVE_SESSION.lock().unwrap();
    if let Some((active_token, _)) = session.as_ref() {
        if active_token == token {
            *session = None;
        }
    }
}

pub fn is_valid_session(token: &str) -> bool {
    let mut session = ACTIVE_SESSION.lock().unwrap();
    
    let mut is_valid = false;
    let mut expired = false;

    if let Some((active_token, last_seen)) = session.as_mut() {
        if active_token == token {
            if last_seen.elapsed() < SESSION_TIMEOUT {
                *last_seen = Instant::now();
                is_valid = true;
            } else {
                expired = true;
            }
        }
    }
    if expired {
        *session = None;
    }
    is_valid
}