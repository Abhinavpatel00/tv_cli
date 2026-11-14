pub mod anime;
pub mod archive;
pub mod cartoon;
pub mod iptv;
pub mod piratebay;
pub mod torrentio;
pub mod torrents_csv;

use crate::models::{ImdbResponse, ImdbResult, SourceItem, StreamSource};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const IMDB_SUG: &str = "https://v3.sg.media-imdb.com/suggestion/x/{}.json";

pub fn search_title(q: &str) -> Result<Vec<ImdbResult>, Box<dyn std::error::Error>> {
    let encoded = urlencoding::encode(q.trim());
    let url = IMDB_SUG.replace("{}", &encoded);
    let text = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(8))
        .call()?
        .into_string()?;
    let resp: ImdbResponse = serde_json::from_str(&text)?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    if let Some(d) = resp.d {
        for r in d {
            if !r.id.starts_with("tt") {
                continue;
            }
            let qid = (r.qid.clone().unwrap_or_default()
                + " "
                + &r.q.clone().unwrap_or_default())
            .to_lowercase();
            if qid.contains("video")
                || qid.contains("short")
                || qid.contains("videogame")
                || qid.contains("podcastseries")
            {
                continue;
            }
            if seen.contains(&r.id) {
                continue;
            }
            seen.insert(r.id.clone());
            out.push(r);
        }
    }
    Ok(out)
}

pub fn fetch_all_streaming_sources(
    ttid: &str,
    clean_title: &str,
    year: &str,
    is_series: bool,
    season: u32,
    episode: u32,
) -> Vec<SourceItem> {
    let (tx, rx) = mpsc::channel::<Vec<SourceItem>>();

    // Worker 1: Torrentio
    {
        let tx = tx.clone();
        let ttid = ttid.to_string();
        thread::spawn(move || {
            let res = torrentio::fetch_torrentio(&ttid, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 2: The Pirate Bay (Apibay)
    {
        let tx = tx.clone();
        let title = clean_title.to_string();
        let yr = year.to_string();
        thread::spawn(move || {
            let res = piratebay::fetch_piratebay(&title, &yr, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 3: Torrents-CSV
    {
        let tx = tx.clone();
        let title = clean_title.to_string();
        let yr = year.to_string();
        thread::spawn(move || {
            let res = torrents_csv::fetch_torrents_csv(&title, &yr, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    drop(tx);

    let mut combined: Vec<SourceItem> = Vec::new();
    // Wait for all 3 worker results
    while let Ok(items) = rx.recv() {
        combined.extend(items);
    }

    // Deduplicate and merge by info_hash
    let mut deduplicated: Vec<SourceItem> = Vec::new();
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for item in combined {
        let hash_opt = match &item.stream {
            StreamSource::Torrent { info_hash, .. } => Some(info_hash.to_lowercase()),
            _ => None,
        };

        if let Some(hash) = hash_opt {
            if let Some(&existing_idx) = seen_hashes.get(&hash) {
                let existing = &mut deduplicated[existing_idx];

                // Merge provider badges if from different source
                if !existing.provider.contains(&item.provider) {
                    existing.provider = format!("{} + {}", existing.provider, item.provider);
                }

                // Update seed count to max
                if let Some(s) = item.seeds {
                    existing.seeds = Some(existing.seeds.unwrap_or(0).max(s));
                }

                // Update file size if existing was missing
                if existing.size_bytes.is_none() && item.size_bytes.is_some() {
                    existing.size_bytes = item.size_bytes;
                    existing.size_str = item.size_str;
                }
                continue;
            }
            seen_hashes.insert(hash, deduplicated.len());
        }

        deduplicated.push(item);
    }

    // Default sort by seeders (highest health first)
    deduplicated.sort_by(|a, b| {
        let seeds_b = b.seeds.unwrap_or(0);
        let seeds_a = a.seeds.unwrap_or(0);
        seeds_b.cmp(&seeds_a)
    });

    // Re-number 1..N
    for (idx, item) in deduplicated.iter_mut().enumerate() {
        item.num = (idx + 1).to_string();
    }

    deduplicated
}
