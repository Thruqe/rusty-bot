#[inline(always)]
pub fn process_memory() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<f64>() {
                            if kb >= 1024.0 * 1024.0 {
                                return format!("{:.2} GB", kb / (1024.0 * 1024.0));
                            } else if kb >= 1024.0 {
                                return format!("{:.2} MB", kb / 1024.0);
                            } else {
                                return format!("{:.2} KB", kb);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            let mut parts = content.split_whitespace();
            if let (Some(_size), Some(resident)) = (parts.next(), parts.next()) {
                if let Ok(pages) = resident.parse::<usize>() {
                    let bytes = pages * 4096;
                    let mb = bytes as f64 / (1024.0 * 1024.0);
                    return format!("{:.2} MB", mb);
                }
            }
        }
    }
    "0 MB".to_string()
}
