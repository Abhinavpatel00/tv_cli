use crate::models::{ArchiveFile, ArchiveItem};
use crate::sources::piratebay::format_size;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct ArchiveSearchDoc {
    identifier: Option<String>,
    title: Option<String>,
    year: Option<serde_json::Value>,
    downloads: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct ArchiveSearchResponse {
    response: Option<ArchiveSearchInner>,
}

#[derive(Deserialize, Debug)]
struct ArchiveSearchInner {
    docs: Option<Vec<ArchiveSearchDoc>>,
}

#[derive(Deserialize, Debug)]
struct ArchiveMetadataFile {
    name: Option<String>,
    size: Option<serde_json::Value>,
    format: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ArchiveMetadataResponse {
    files: Option<Vec<ArchiveMetadataFile>>,
}

pub fn search_archive(query: &str) -> Result<Vec<ArchiveItem>, String> {
    let clean_kw: String = query
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect();
    let words: Vec<&str> = clean_kw.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }

    let search_title = words.join(" AND ");
    let q_param = format!("mediatype:movies AND title:({search_title})");
    let enc_q = urlencoding::encode(&q_param);

    let url = format!(
        "https://archive.org/advancedsearch.php?q={enc_q}&fl[]=identifier&fl[]=title&fl[]=year&fl[]=downloads&sort[]=downloads%20desc&rows=30&output=json"
    );

    let resp = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| format!("Archive search failed: {e}"))?;

    let body = resp
        .into_string()
        .map_err(|e| format!("Failed to read archive response: {e}"))?;

    let parsed: ArchiveSearchResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse archive response: {e}"))?;

    let docs = parsed
        .response
        .and_then(|r| r.docs)
        .unwrap_or_default();

    let mut out = Vec::new();
    for d in docs {
        let identifier = match d.identifier {
            Some(i) if !i.is_empty() => i,
            _ => continue,
        };
        let title = d.title.unwrap_or_else(|| identifier.clone());
        let t_lower = title.to_lowercase();
        if t_lower.contains("trailer")
            || t_lower.contains("promo")
            || t_lower.contains("sample")
            || t_lower.contains("teaser")
        {
            continue;
        }

        let year = match d.year {
            Some(serde_json::Value::String(s)) => s,
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => "----".to_string(),
        };

        out.push(ArchiveItem {
            identifier,
            title,
            year,
            downloads: d.downloads.unwrap_or(0),
        });
    }

    Ok(out)
}

pub fn fetch_archive_files(identifier: &str) -> Result<Vec<ArchiveFile>, String> {
    let url = format!("https://archive.org/metadata/{identifier}");

    let resp = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| format!("Failed to fetch files for {identifier}: {e}"))?;

    let body = resp
        .into_string()
        .map_err(|e| format!("Failed to read metadata: {e}"))?;

    let meta: ArchiveMetadataResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse metadata: {e}"))?;

    let files = meta.files.unwrap_or_default();
    let mut out = Vec::new();

    for f in files {
        let name = match f.name {
            Some(n) => n,
            None => continue,
        };

        let nl = name.to_lowercase();
        if !nl.ends_with(".mp4")
            && !nl.ends_with(".mkv")
            && !nl.ends_with(".webm")
            && !nl.ends_with(".ogv")
            && !nl.ends_with(".m4v")
        {
            continue;
        }

        let size_bytes: u64 = match f.size {
            Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(serde_json::Value::String(s)) => s.parse::<u64>().unwrap_or(0),
            _ => 0,
        };

        let size_str = if size_bytes > 0 {
            format_size(size_bytes)
        } else {
            "N/A".to_string()
        };

        let enc_name = urlencoding::encode(&name);
        let stream_url = format!("https://archive.org/download/{identifier}/{enc_name}");

        out.push(ArchiveFile {
            name,
            size_str,
            size_bytes,
            url: stream_url,
            format: f.format.unwrap_or_else(|| "Video".to_string()),
        });
    }

    // Sort by largest video file first (usually best quality)
    out.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));

    Ok(out)
}
