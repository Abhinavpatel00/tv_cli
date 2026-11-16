use crate::models::{SourceItem, StreamSource};
use serde::Deserialize;
use std::env;
use std::process::Command;
use std::time::Duration;

const SSS: &str = "./sss.txt";
const DEFAULT_TORRENTIO: &str = "https://torrentio.strem.fun";

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

pub fn make_magnet(info_hash: &str, title: &str, extra_trackers: &[String]) -> String {
    let mut magnet = format!(
        "magnet:?xt=urn:btih:{}&dn={}",
        info_hash.to_lowercase(),
        urlencoding::encode(title)
    );

    let default_trackers = [
        "udp://tracker.opentrackr.org:1337/announce",
        "udp://open.stealth.si:80/announce",
        "udp://tracker.tiny-vps.com:6969/announce",
        "udp://tracker.torrent.eu.org:451/announce",
        "udp://exodus.desync.com:6969/announce",
        "udp://tracker.openbittorrent.com:6969/announce",
        "udp://opentor.net:6969",
    ];

    let mut seen = std::collections::HashSet::new();
    for tr in extra_trackers {
        let clean = tr.trim().trim_start_matches("tracker:").trim();
        if !clean.is_empty() && seen.insert(clean.to_string()) {
            magnet.push_str("&tr=");
            magnet.push_str(&urlencoding::encode(clean));
        }
    }
    for tr in default_trackers {
        if seen.insert(tr.to_string()) {
            magnet.push_str("&tr=");
            magnet.push_str(&urlencoding::encode(tr));
        }
    }

    magnet
}

pub fn parse_release_metadata(text: &str) -> (String, Option<String>, Option<String>) {
    let t = text.to_lowercase();

    let resolution = if t.contains("4k") || t.contains("2160p") || t.contains("uhd") {
        "4K".to_string()
    } else if t.contains("1080p") || t.contains("1080i") || t.contains("fhd") {
        "1080p".to_string()
    } else if t.contains("720p") || t.contains("hd") {
        "720p".to_string()
    } else if t.contains("480p") || t.contains("576p") || t.contains("sd") {
        "480p".to_string()
    } else {
        "Other".to_string()
    };

    let hdr_info = if t.contains("dv") || t.contains("dolby vision") || t.contains("dovi") {
        Some("DV/HDR".to_string())
    } else if t.contains("hdr10+") {
        Some("HDR10+".to_string())
    } else if t.contains("hdr") {
        Some("HDR".to_string())
    } else {
        None
    };

    let codec = if t.contains("x265") || t.contains("h265") || t.contains("hevc") || t.contains("10bit") {
        Some("HEVC/x265".to_string())
    } else if t.contains("x264") || t.contains("h264") || t.contains("avc") {
        Some("x264".to_string())
    } else if t.contains("av1") {
        Some("AV1".to_string())
    } else {
        None
    };

    (resolution, hdr_info, codec)
}

pub fn fetch_torrentio(
    ttid: &str,
    is_series: bool,
    season: u32,
    episode: u32,
) -> Vec<SourceItem> {
    let base_url = env::var("SSS_TORRENTIO").unwrap_or_else(|_| DEFAULT_TORRENTIO.to_string());
    let endpoint = if is_series {
        format!("{base_url}/stream/series/{ttid}:{season}:{episode}.json")
    } else {
        format!("{base_url}/stream/movie/{ttid}.json")
    };

    if let Ok(resp) = ureq::get(&endpoint)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .set("Origin", "https://web.stremio.com")
        .set("Referer", "https://web.stremio.com/")
        .timeout(Duration::from_secs(6))
        .call()
    {
        if let Ok(body) = resp.into_string() {
            if let Ok(parsed) = serde_json::from_str::<TorrentioResponse>(&body) {
                if let Some(streams) = parsed.streams {
                    let mut items = Vec::new();
                    for (idx, s) in streams.into_iter().enumerate() {
                        let info_hash = match s.info_hash {
                            Some(h) if !h.is_empty() => h,
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
                                let parts: Vec<&str> = s_clean.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    if let Ok(val) = parts[0].parse::<f64>() {
                                        let unit = parts[1].to_uppercase();
                                        let mult = if unit.contains("TB") {
                                            1024.0 * 1024.0 * 1024.0 * 1024.0
                                        } else if unit.contains("GB") {
                                            1024.0 * 1024.0 * 1024.0
                                        } else if unit.contains("MB") {
                                            1024.0 * 1024.0
                                        } else {
                                            1024.0
                                        };
                                        size_bytes = Some((val * mult) as u64);
                                    }
                                }
                            }
                        }

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
                            num: (idx + 1).to_string(),
                            provider: "Torrentio".to_string(),
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

                    if !items.is_empty() {
                        return items;
                    }
                }
            }
        }
    }

    // Fallback to sss.txt subprocess if direct HTTP had an issue
    fallback_list_sources(ttid, if is_series { Some((season, episode)) } else { None })
}

pub fn parse_source_line(line: &str) -> Option<SourceItem> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (num_part, rest) = line.split_once('.')?;
    let num_part = num_part.trim();
    if !num_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let rest = rest.trim();
    let parts: Vec<&str> = rest.split('|').map(|s| s.trim()).collect();

    let mut provider = "Torrentio".to_string();
    let mut resolution = "Other".to_string();
    let mut hdr_info = None;
    let mut codec = None;
    let mut seeds = None;
    let mut size_str = "N/A".to_string();
    let mut size_bytes = None;
    let mut title = rest.to_string();

    if parts.len() >= 3 {
        let p0 = parts[0];
        let p0_lower = p0.to_lowercase();
        if p0_lower.contains("torrentio") {
            provider = "Torrentio".to_string();
        }

        if p0_lower.contains("4k") || p0_lower.contains("2160p") {
            resolution = "4K".to_string();
        } else if p0_lower.contains("1080p") {
            resolution = "1080p".to_string();
        } else if p0_lower.contains("720p") {
            resolution = "720p".to_string();
        } else if p0_lower.contains("480p") {
            resolution = "480p".to_string();
        }

        if p0_lower.contains("dv") || p0_lower.contains("hdr") {
            hdr_info = Some("DV/HDR".to_string());
        }

        for part in &parts {
            let pl = part.to_lowercase();
            if pl.starts_with("seed:") {
                let s_str = pl.trim_start_matches("seed:").trim();
                seeds = s_str.parse::<u32>().ok();
            } else if pl.contains("gb") || pl.contains("mb") {
                size_str = part.to_string();
                let subparts: Vec<&str> = part.split_whitespace().collect();
                if subparts.len() >= 2 {
                    if let Ok(val) = subparts[0].parse::<f64>() {
                        let unit = subparts[1].to_uppercase();
                        let mult = if unit.contains("GB") {
                            1024.0 * 1024.0 * 1024.0
                        } else {
                            1024.0 * 1024.0
                        };
                        size_bytes = Some((val * mult) as u64);
                    }
                }
            } else if pl == "hdr" || pl == "dv" || pl.contains("hdr") || pl.contains("dv") {
                hdr_info = Some(part.to_string());
            } else if pl.contains("hevc") || pl.contains("x265") {
                codec = Some("HEVC/x265".to_string());
            } else if pl.contains("x264") {
                codec = Some("x264".to_string());
            }
        }

        if let Some(last) = parts.last() {
            if !last.starts_with("seed:") && !last.contains("GB") && !last.contains("MB") {
                title = last.to_string();
            }
        }
    }

    Some(SourceItem {
        num: num_part.to_string(),
        provider,
        resolution,
        hdr_info,
        codec,
        seeds,
        size_bytes,
        size_str,
        title,
        stream: StreamSource::SssPick {
            num: num_part.to_string(),
        },
    })
}

fn fallback_list_sources(
    ttid: &str,
    episode_info: Option<(u32, u32)>,
) -> Vec<SourceItem> {
    let sss_path = crate::player::find_sss_executable()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| SSS.to_string());
    let mut cmd = Command::new(&sss_path);
    cmd.arg("id").arg(ttid);

    if let Some((season, episode)) = episode_info {
        cmd.arg(season.to_string()).arg(episode.to_string());
    }
    cmd.arg("--list").arg("--no-subs");

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut srcs = Vec::new();
    for line in text.lines() {
        if let Some(item) = parse_source_line(line) {
            srcs.push(item);
        }
    }
    srcs
}
