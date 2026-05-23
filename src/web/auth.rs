use core::sync::atomic::{AtomicBool, Ordering};

static LOGGED_IN: AtomicBool = AtomicBool::new(false);

pub fn login_ok() {
    LOGGED_IN.store(true, Ordering::SeqCst);
}

pub fn logout() {
    LOGGED_IN.store(false, Ordering::SeqCst);
}

pub fn is_logged_in() -> bool {
    LOGGED_IN.load(Ordering::SeqCst)
}