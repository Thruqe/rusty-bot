use std::sync::LazyLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

#[inline(always)]
pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock drift")
        .as_millis()
}

pub fn format_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0s".to_string();
    }

    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if secs > 0 {
        parts.push(format!("{secs}s"));
    }

    parts.join(" ")
}

#[inline(always)]
pub fn uptime() -> String {
    format_duration(START_TIME.elapsed().as_secs())
}
