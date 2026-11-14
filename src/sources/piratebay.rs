use crate::models::{SourceItem, StreamSource};
use crate::sources::torrentio::{make_magnet, parse_release_metadata};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct ApibayItem {
    id: String,
    name: String,
    info_hash: String,
    seeders: String,
    size: String,
}

pub fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn fetch_piratebay(
    clean_title: &str,
    year: &str,
    is_series: bool,
    season: u32,
    episode: u32,
) -> Vec<SourceItem> {
    let query = if is_series {
        format!("{clean_title} S{:02}E{:02}", season, episode)
    } else if !year.is_empty() && year != "----" {
        format!("{clean_title} {year}")
    } else {
        clean_title.to_string()
    };

    let encoded = urlencoding::encode(&query);
    let url = format!("https://apibay.org/q.php?q={encoded}");

    let resp = match ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(5))
        .call()
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let body = match resp.into_string() {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };

    let items: Vec<ApibayItem> = match serde_json::from_str(&body) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for (idx, item) in items.into_iter().enumerate() {
        if item.id == "0" || item.name == "No results returned" {
            continue;
        }

        let info_hash = item.info_hash.trim().to_lowercase();
        if info_hash.is_empty() || info_hash == "0000000000000000000000000000000000000000" {
            continue;
        }

        let seeds = item.seeders.parse::<u32>().ok();
        let bytes = item.size.parse::<u64>().ok();
        let size_str = bytes.map(format_size).unwrap_or_else(|| "N/A".to_string());

        let (resolution, hdr_info, codec) = parse_release_metadata(&item.name);
        let magnet = make_magnet(&info_hash, &item.name, &[]);

        out.push(SourceItem {
            num: (idx + 1).to_string(),
            provider: "The Pirate Bay".to_string(),
            resolution,
            hdr_info,
            codec,
            seeds,
            size_bytes: bytes,
            size_str,
            title: item.name,
            stream: StreamSource::Torrent {
                magnet,
                info_hash,
                file_idx: None,
            },
        });
    }

    out
}
