use crate::models::{KitsuAnime, SourceItem, StreamSource};
use crate::sources::piratebay;
use crate::sources::torrentio::{make_magnet, parse_release_metadata};
use crate::sources::torrents_csv;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const KITSU_API_BASE: &str = "https://kitsu.io/api/edge/anime";
const NYAA_RSS_BASE: &str = "https://nyaa.si/?page=rss";
const DEFAULT_TORRENTIO: &str = "https://torrentio.strem.fun";

// ----------------------------------------------------------------------------
// Kitsu JSON API Models
// ----------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct KitsuPosterImage {
    tiny: Option<String>,
    small: Option<String>,
    medium: Option<String>,
    large: Option<String>,
}

#[derive(Deserialize, Debug)]
struct KitsuTitles {
    en: Option<String>,
    en_jp: Option<String>,
    ja_jp: Option<String>,
}

#[derive(Deserialize, Debug)]
struct KitsuAttributes {
    #[serde(rename = "canonicalTitle")]
    canonical_title: Option<String>,
    titles: Option<KitsuTitles>,
    synopsis: Option<String>,
    #[serde(rename = "episodeCount")]
    episode_count: Option<u32>,
    subtype: Option<String>,
    status: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "averageRating")]
    average_rating: Option<String>,
    #[serde(rename = "ageRating")]
    age_rating: Option<String>,
    #[serde(rename = "ageRatingGuide")]
    age_rating_guide: Option<String>,
    #[serde(rename = "posterImage")]
    #[allow(dead_code)]
    poster_image: Option<KitsuPosterImage>,
}

#[derive(Deserialize, Debug)]
struct KitsuData {
    id: String,
    attributes: KitsuAttributes,
}

#[derive(Deserialize, Debug)]
struct KitsuResponse {
    data: Option<Vec<KitsuData>>,
}

impl From<KitsuData> for KitsuAnime {
    fn from(d: KitsuData) -> Self {
        let title = d
            .attributes
            .canonical_title
            .unwrap_or_else(|| "Unknown Anime".to_string());
        let (title_en, title_jp) = if let Some(t) = d.attributes.titles {
            (t.en, t.ja_jp.or(t.en_jp))
        } else {
            (None, None)
        };

        let year = d
            .attributes
            .start_date
            .as_deref()
            .map(|s| {
                if s.len() >= 4 {
                    s[0..4].to_string()
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| "----".to_string());

        let synopsis = d
            .attributes
            .synopsis
            .unwrap_or_else(|| "No synopsis available.".to_string());

        KitsuAnime {
            id: d.id,
            title,
            title_en,
            title_jp,
            synopsis,
            episode_count: d.attributes.episode_count,
            subtype: d.attributes.subtype.unwrap_or_else(|| "TV".to_string()),
            status: d.attributes.status.unwrap_or_else(|| "Unknown".to_string()),
            year,
            rating: d.attributes.average_rating,
            age_rating_guide: d.attributes.age_rating_guide.or(d.attributes.age_rating),
        }
    }
}

pub fn search_anime(query: &str) -> Result<Vec<KitsuAnime>, Box<dyn std::error::Error>> {
    let q = query.trim();
    let url = if q.is_empty() {
        format!("{KITSU_API_BASE}?sort=-user_count&page[limit]=20")
    } else {
        let encoded = urlencoding::encode(q);
        format!("{KITSU_API_BASE}?filter[text]={encoded}&page[limit]=20")
    };

    let text = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(8))
        .call()?
        .into_string()?;

    let resp: KitsuResponse = serde_json::from_str(&text)?;
    let mut out = Vec::new();
    if let Some(items) = resp.data {
        for item in items {
            out.push(KitsuAnime::from(item));
        }
    }
    Ok(out)
}

// ----------------------------------------------------------------------------
// Torrentio Anime Streaming Provider
// ----------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct TorrentioBehaviorHints {
    filename: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TorrentioStream {
    name: Option<String>,
    title: Option<String>,
    #[serde(rename = "infoHash")]
    info_hash: Option<String>,
    #[serde(rename = "fileIdx")]
    file_idx: Option<u32>,
    #[serde(rename = "behaviorHints")]
    behavior_hints: Option<TorrentioBehaviorHints>,
    sources: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct TorrentioResponse {
    streams: Option<Vec<TorrentioStream>>,
}

pub fn fetch_torrentio_anime(kitsu_id: &str, is_movie: bool, episode: u32) -> Vec<SourceItem> {
    let base_url = env::var("SSS_TORRENTIO").unwrap_or_else(|_| DEFAULT_TORRENTIO.to_string());
    let endpoint = if is_movie {
        format!("{base_url}/stream/movie/kitsu:{kitsu_id}.json")
    } else {
        format!("{base_url}/stream/series/kitsu:{kitsu_id}:{episode}.json")
    };

    let text = match ureq::get(&endpoint)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .set("Origin", "https://web.stremio.com")
        .set("Referer", "https://web.stremio.com/")
        .timeout(Duration::from_secs(6))
        .call()
    {
        Ok(resp) => match resp.into_string() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    let parsed = match serde_json::from_str::<TorrentioResponse>(&text) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut items = Vec::new();
    if let Some(streams) = parsed.streams {
        for s in streams {
            let info_hash = match s.info_hash {
                Some(h) if !h.is_empty() => h.to_lowercase(),
                _ => continue,
            };

            let name = s.name.unwrap_or_default();
            let title = s.title.unwrap_or_default();
            let full_desc = format!("{name} {title}");

            let (resolution, hdr_info, codec) = parse_release_metadata(&full_desc);

            let mut seeds = None;
            if let Some(seed_pos) = title.find('👤') {
                let after = &title[seed_pos + '👤'.len_utf8()..];
                let seed_str: String = after
                    .chars()
                    .skip_while(|c| c.is_whitespace())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                seeds = seed_str.parse::<u32>().ok();
            }

            let mut size_str = "N/A".to_string();
            let mut size_bytes = None;
            if let Some(size_pos) = title.find('💾') {
                let after = &title[size_pos + '💾'.len_utf8()..];
                let s_str: String = after
                    .chars()
                    .skip_while(|c| c.is_whitespace())
                    .take_while(|c| !['⚙', '👤', '\n', '|'].contains(c))
                    .collect();
                let s_clean = s_str.trim().to_string();
                if !s_clean.is_empty() {
                    size_str = s_clean.clone();
                    size_bytes = parse_size_string(&s_clean);
                }
            }

            // Identify indexer provider from gear symbol
            let provider = if let Some(p_pos) = title.find('⚙') {
                let after = &title[p_pos + '⚙'.len_utf8()..];
                let p_str: String = after
                    .chars()
                    .skip_while(|c| c.is_whitespace())
                    .take_while(|c| !['\n', '|', '👤', '💾'].contains(c))
                    .collect();
                let p_clean = p_str.trim().to_string();
                if !p_clean.is_empty() {
                    format!("Torrentio ({p_clean})")
                } else {
                    "Torrentio".to_string()
                }
            } else {
                "Torrentio".to_string()
            };

            let clean_title = if let Some(bh) = s.behavior_hints {
                bh.filename.unwrap_or_else(|| {
                    title.lines().next().unwrap_or(&title).to_string()
                })
            } else {
                title.lines().next().unwrap_or(&title).to_string()
            };

            let extra_trackers = s.sources.unwrap_or_default();
            let magnet = make_magnet(&info_hash, &clean_title, &extra_trackers);

            items.push(SourceItem {
                num: String::new(),
                provider,
                resolution,
                hdr_info,
                codec,
                seeds,
                size_bytes,
                size_str,
                title: clean_title,
                stream: StreamSource::Torrent {
                    magnet,
                    info_hash,
                    file_idx: s.file_idx,
                },
            });
        }
    }

    items
}

// ----------------------------------------------------------------------------
// Nyaa.si Direct RSS Scraper
// ----------------------------------------------------------------------------

pub fn fetch_nyaa_anime(clean_title: &str, is_movie: bool, episode: u32) -> Vec<SourceItem> {
    let query_str = if is_movie {
        clean_title.to_string()
    } else {
        format!("{} {:02}", clean_title, episode)
    };
    let encoded = urlencoding::encode(&query_str);
    let url = format!("{NYAA_RSS_BASE}&q={encoded}&c=1_2&s=seeders&o=desc");

    let text = match ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(6))
        .call()
    {
        Ok(resp) => match resp.into_string() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    parse_nyaa_rss(&text, is_movie, episode)
}

pub fn parse_nyaa_rss(xml: &str, is_movie: bool, target_episode: u32) -> Vec<SourceItem> {
    let mut items = Vec::new();
    let parts: Vec<&str> = xml.split("<item>").collect();
    if parts.len() <= 1 {
        return items;
    }

    let nyaa_trackers = [
        "udp://tracker.nyaa.vc:6969/announce".to_string(),
        "http://nyaa.tracker.wf:7777/announce".to_string(),
        "http://anidex.moe:6969/announce".to_string(),
        "udp://tracker.opentrackr.org:1337/announce".to_string(),
        "udp://open.stealth.si:80/announce".to_string(),
    ];

    for part in parts.into_iter().skip(1) {
        let end_idx = match part.find("</item>") {
            Some(i) => i,
            None => continue,
        };
        let item_xml = &part[..end_idx];

        let title = extract_xml_tag(item_xml, "title").unwrap_or_default();
        if title.is_empty() {
            continue;
        }

        let info_hash = match extract_xml_tag(item_xml, "nyaa:infoHash") {
            Some(h) if h.len() == 40 => h.to_lowercase(),
            _ => continue,
        };

        let seeders = extract_xml_tag(item_xml, "nyaa:seeders")
            .and_then(|s| s.trim().parse::<u32>().ok());

        let size_str = extract_xml_tag(item_xml, "nyaa:size")
            .unwrap_or_else(|| "N/A".to_string())
            .trim()
            .to_string();

        let size_bytes = parse_size_string(&size_str);

        if !is_movie && !title_matches_episode(&title, target_episode) {
            continue;
        }

        let (resolution, hdr_info, codec) = parse_release_metadata(&title);

        let release_group = if title.starts_with('[') {
            if let Some(end_bracket) = title.find(']') {
                let group = &title[1..end_bracket].trim();
                if !group.is_empty() && group.len() < 25 {
                    Some(group.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let provider = match release_group {
            Some(g) => format!("NyaaSi ({g})"),
            None => "NyaaSi".to_string(),
        };

        let magnet = make_magnet(&info_hash, &title, &nyaa_trackers);

        items.push(SourceItem {
            num: String::new(),
            provider,
            resolution,
            hdr_info,
            codec,
            seeds: seeders,
            size_bytes,
            size_str,
            title,
            stream: StreamSource::Torrent {
                magnet,
                info_hash,
                file_idx: None,
            },
        });
    }

    items
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)? + start;
    let raw = &xml[start..end];
    Some(unescape_xml(raw.trim()))
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub fn parse_size_string(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(val) = parts[0].parse::<f64>() {
            let unit = parts[1].to_uppercase();
            let mult = if unit.contains("TIB") || unit.contains("TB") {
                1024.0 * 1024.0 * 1024.0 * 1024.0
            } else if unit.contains("GIB") || unit.contains("GB") {
                1024.0 * 1024.0 * 1024.0
            } else if unit.contains("MIB") || unit.contains("MB") {
                1024.0 * 1024.0
            } else if unit.contains("KIB") || unit.contains("KB") {
                1024.0
            } else {
                1.0
            };
            return Some((val * mult) as u64);
        }
    }
    None
}

pub fn title_matches_episode(title: &str, target_episode: u32) -> bool {
    let lower = title.to_lowercase();

    // Check batch releases that contain this episode
    if lower.contains("batch") || lower.contains("(01-") || lower.contains("01~") {
        return true;
    }

    // Strip trailing CRC32 bracket like [F502B6D4]
    let clean = if let Some(last_open) = lower.rfind('[') {
        if let Some(last_close) = lower[last_open..].find(']') {
            let inner = &lower[last_open + 1..last_open + last_close];
            if inner.len() == 8 && inner.chars().all(|c| c.is_ascii_hexdigit()) {
                let mut s = lower.clone();
                s.replace_range(last_open..=last_open + last_close, "");
                s
            } else {
                lower
            }
        } else {
            lower
        }
    } else {
        lower
    };

    let ep2 = format!("{:02}", target_episode);
    let p1 = format!(" - {ep2}");
    let p2 = format!(" {ep2} ");
    let p3 = format!(" {ep2}[");
    let p4 = format!(" {ep2}(");
    let p5 = format!("e{ep2}");
    let p6 = format!("ep{ep2}");
    let p7 = format!("ep {ep2}");
    let p8 = format!("episode {ep2}");
    let p9 = format!(" - {} ", target_episode);
    let p10 = format!(" - {}[", target_episode);
    let p11 = format!(" - {ep2}.");

    clean.contains(&p1)
        || clean.contains(&p2)
        || clean.contains(&p3)
        || clean.contains(&p4)
        || clean.contains(&p5)
        || clean.contains(&p6)
        || clean.contains(&p7)
        || clean.contains(&p8)
        || clean.contains(&p9)
        || clean.contains(&p10)
        || clean.contains(&p11)
}

// ----------------------------------------------------------------------------
// Multi-Source Parallel Aggregator for Anime
// ----------------------------------------------------------------------------

pub fn fetch_all_anime_sources(anime: &KitsuAnime, episode: u32) -> Vec<SourceItem> {
    let (tx, rx) = mpsc::channel::<Vec<SourceItem>>();
    let is_movie = anime.is_movie();

    // Worker 1: Torrentio Anime (Kitsu catalog)
    {
        let tx = tx.clone();
        let kitsu_id = anime.id.clone();
        thread::spawn(move || {
            let res = fetch_torrentio_anime(&kitsu_id, is_movie, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 2: Nyaa.si direct RSS (using canonical title)
    {
        let tx = tx.clone();
        let title = anime.title.clone();
        thread::spawn(move || {
            let res = fetch_nyaa_anime(&title, is_movie, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 3: Nyaa.si direct RSS (using English title if different from canonical)
    if let Some(en) = &anime.title_en {
        if !en.is_empty() && !en.eq_ignore_ascii_case(&anime.title) {
            let tx = tx.clone();
            let en_title = en.clone();
            thread::spawn(move || {
                let res = fetch_nyaa_anime(&en_title, is_movie, episode);
                let _ = tx.send(res);
            });
        }
    }

    // Worker 4: The Pirate Bay (Apibay)
    {
        let tx = tx.clone();
        let title = anime.display_title().to_string();
        let yr = anime.year.clone();
        thread::spawn(move || {
            let res = piratebay::fetch_piratebay(&title, &yr, !is_movie, 1, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 5: Torrents-CSV
    {
        let tx = tx.clone();
        let title = anime.display_title().to_string();
        let yr = anime.year.clone();
        thread::spawn(move || {
            let res = torrents_csv::fetch_torrents_csv(&title, &yr, !is_movie, 1, episode);
            let _ = tx.send(res);
        });
    }

    drop(tx);

    let mut combined: Vec<SourceItem> = Vec::new();
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

                if !existing.provider.contains(&item.provider) {
                    existing.provider = format!("{} + {}", existing.provider, item.provider);
                }

                if let Some(s) = item.seeds {
                    existing.seeds = Some(existing.seeds.unwrap_or(0).max(s));
                }

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

    // Default sort by seeders (highest swarm health first)
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
