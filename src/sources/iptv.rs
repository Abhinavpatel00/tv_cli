use crate::models::IptvChannel;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const IPTV_URL: &str = "https://iptv-org.github.io/iptv/index.m3u";

pub fn iptv_cache_path() -> PathBuf {
    let cache_dir = env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".cache")
        });
    cache_dir.join("sss").join("iptv.m3u")
}

pub fn load_or_fetch_channels() -> Result<Vec<IptvChannel>, String> {
    let path = iptv_cache_path();

    let mut needs_download = true;
    if path.exists() {
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed < Duration::from_secs(86400) && metadata.len() > 100_000 {
                        needs_download = false;
                    }
                }
            }
        }
    }

    if needs_download {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(resp) = ureq::get(IPTV_URL)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .timeout(Duration::from_secs(15))
            .call()
        {
            if let Ok(content) = resp.into_string() {
                if content.len() > 50_000 {
                    let _ = fs::write(&path, &content);
                }
            }
        }
    }

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            // Attempt direct download if file read failed
            let resp = ureq::get(IPTV_URL)
                .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .timeout(Duration::from_secs(12))
                .call()
                .map_err(|e| format!("Failed to download IPTV channels: {e}"))?;
            resp.into_string()
                .map_err(|e| format!("Failed to read IPTV response: {e}"))?
        }
    };

    Ok(parse_m3u(&text))
}

pub fn parse_m3u(content: &str) -> Vec<IptvChannel> {
    let mut channels = Vec::new();
    let mut cur_group = "General".to_string();
    let mut cur_name = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("#EXTINF") {
            if let Some(grp_pos) = line.find("group-title=\"") {
                let after = &line[grp_pos + "group-title=\"".len()..];
                if let Some(end_quote) = after.find('"') {
                    let raw_grp = &after[..end_quote];
                    cur_group = clean_group_name(raw_grp);
                } else {
                    cur_group = "General".to_string();
                }
            } else {
                cur_group = "General".to_string();
            }

            if let Some(comma_pos) = line.rfind(',') {
                cur_name = line[comma_pos + 1..].trim().to_string();
            } else {
                cur_name = "Unknown Channel".to_string();
            }
        } else if (line.starts_with("http://") || line.starts_with("https://"))
            && !cur_name.is_empty() {
                let resolution = extract_resolution(&cur_name);
                channels.push(IptvChannel {
                    group: cur_group.clone(),
                    name: cur_name.clone(),
                    url: line.to_string(),
                    resolution,
                });
                cur_name.clear();
            }
    }

    channels
}

fn clean_group_name(raw: &str) -> String {
    let parts: Vec<&str> = raw.split(';').map(|s| s.trim()).collect();
    if let Some(&first) = parts.first() {
        if !first.is_empty() {
            return first.to_string();
        }
    }
    "General".to_string()
}

fn extract_resolution(name: &str) -> String {
    let nl = name.to_lowercase();
    if nl.contains("1080p") || nl.contains("fhd") {
        "1080p".to_string()
    } else if nl.contains("720p") || nl.contains("hd") {
        "720p".to_string()
    } else if nl.contains("4k") || nl.contains("2160p") {
        "4K".to_string()
    } else if nl.contains("480p") || nl.contains("576p") || nl.contains("sd") {
        "SD".to_string()
    } else {
        "Live".to_string()
    }
}
