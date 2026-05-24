use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

lazy_static::lazy_static! {
    static ref SESSIONS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
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
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.insert(token.clone(), Instant::now());
    token
}

pub fn logout(token: &str) {
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.remove(token);
}

pub fn is_valid_session(token: &str) -> bool {
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.retain(|_, last_seen| last_seen.elapsed() < SESSION_TIMEOUT);
    if let Some(last_seen) = sessions.get_mut(token) {
        *last_seen = Instant::now();
        true
    } else {
        false
    }
}