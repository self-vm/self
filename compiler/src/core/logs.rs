use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    static ref LOG_HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

pub fn log(str: &str) {
    println!("{str}");

    LOG_HISTORY.lock().unwrap().push(str.to_string());
}

pub fn get_log_history() -> Vec<String> {
    LOG_HISTORY.lock().unwrap().clone()
}

// Macro to simplify logging usage
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::core::logs::log(&format!($($arg)*));
    };
}
