use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn write_log(logtype: &str, data: &String) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let filename = format!(".logs/{}_{}.txt", logtype, now);
    let _ = fs::write(&filename, data.clone());
}
