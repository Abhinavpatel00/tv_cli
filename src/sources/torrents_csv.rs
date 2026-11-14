use crate::models::{SourceItem, StreamSource};
use crate::sources::piratebay::format_size;
use crate::sources::torrentio::{make_magnet, parse_release_metadata};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct CsvTorrent {
    infohash: String,
    name: String,
    size_bytes: Option<u64>,
    seeders: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct CsvResponse {
    torrents: Option<Vec<CsvTorrent>>,
}

pub fn fetch_torrents_csv(
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
    let url = format!("https://torrents-csv.com/service/search?q={encoded}");

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

    let parsed: CsvResponse = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let torrents = match parsed.torrents {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut out = Vec::new();
    for (idx, item) in torrents.into_iter().enumerate() {
        let info_hash = item.infohash.trim().to_lowercase();
        if info_hash.is_empty() {
            continue;
        }

        let size_str = item.size_bytes.map(format_size).unwrap_or_else(|| "N/A".to_string());
        let (resolution, hdr_info, codec) = parse_release_metadata(&item.name);
        let magnet = make_magnet(&info_hash, &item.name, &[]);

        out.push(SourceItem {
            num: (idx + 1).to_string(),
            provider: "Torrents-CSV".to_string(),
            resolution,
            hdr_info,
            codec,
            seeds: item.seeders,
            size_bytes: item.size_bytes,
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
