use crate::models::{HistoryEntry, SourceItem, StreamSource};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const OPENSUBS: &str = "https://opensubtitles-v3.strem.io/subtitles";

pub fn get_history_file_path() -> PathBuf {
    if let Ok(path) = env::var("TSSS_HISTORY_FILE") {
        return PathBuf::from(path);
    }
    let base = env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("tsss-tui").join("history.json")
}

pub fn load_history() -> Vec<HistoryEntry> {
    let path = get_history_file_path();
    if !path.exists() {
        return Vec::new();
    }
    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_history(entries: &[HistoryEntry]) -> io::Result<()> {
    let path = get_history_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let to_save = if entries.len() > 100 {
        &entries[..100]
    } else {
        entries
    };
    let json = serde_json::to_string_pretty(to_save)
        .map_err(io::Error::other)?;
    fs::write(&path, json)
}

pub fn find_sss_executable() -> Option<PathBuf> {
    if let Ok(p) = env::var("SSS_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    // 1. Check relative to current working directory
    for candidate in &["./sss.txt", "./sss", "../sss.txt", "../sss"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            if let Ok(abs) = path.canonicalize() {
                return Some(abs);
            }
            return Some(path);
        }
    }

    // 2. Check relative to executable binary
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            for candidate in &["sss.txt", "sss", "../sss.txt", "../../sss.txt"] {
                let path = exe_dir.join(candidate);
                if path.is_file() {
                    if let Ok(abs) = path.canonicalize() {
                        return Some(abs);
                    }
                    return Some(path);
                }
            }
        }
    }

    // 3. Known project paths
    for candidate in &[
        "/home/lk/webseries/tsss-tui/sss.txt",
        "/home/lk/webseries/sss.txt",
        "/usr/local/bin/sss",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    // 4. System PATH
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let full = dir.join("sss");
            if full.is_file() {
                return Some(full);
            }
            let full_txt = dir.join("sss.txt");
            if full_txt.is_file() {
                return Some(full_txt);
            }
        }
    }

    None
}

pub fn fetch_subtitles_for_id(ttid: &str, ep_info: Option<(u32, u32)>) -> Option<PathBuf> {
    let lang = env::var("SSS_SUBS").unwrap_or_else(|_| "eng".to_string());
    if lang == "off" || lang.is_empty() {
        return None;
    }

    let cache_dir = env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".cache")
        })
        .join("sss")
        .join("subs");

    let _ = fs::create_dir_all(&cache_dir);

    let (keypath, filename) = if let Some((s, e)) = ep_info {
        (
            format!("series/{ttid}:{s}:{e}.json"),
            format!("{ttid}_s{s}e{e}_{lang}.srt"),
        )
    } else {
        (format!("movie/{ttid}.json"), format!("{ttid}_{lang}.srt"))
    };

    let dest = cache_dir.join(filename);
    if dest.exists() && fs::metadata(&dest).map(|m| m.len() > 0).unwrap_or(false) {
        return Some(dest);
    }

    let url = format!("{OPENSUBS}/{keypath}");
    let resp = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(6))
        .call()
        .ok()?;

    let body = resp.into_string().ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&body).ok()?;
    let subs = parsed.get("subtitles")?.as_array()?;

    let mut download_url = None;
    for s in subs {
        if s.get("lang").and_then(|l| l.as_str()) == Some(&lang) {
            if let Some(u) = s.get("url").and_then(|u| u.as_str()) {
                download_url = Some(u.to_string());
                break;
            }
        }
    }

    if download_url.is_none() {
        if let Some(first) = subs.first() {
            download_url = first.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
        }
    }

    let dl = download_url?;
    let sub_data = ureq::get(&dl)
        .timeout(Duration::from_secs(6))
        .call()
        .ok()?
        .into_string()
        .ok()?;

    if !sub_data.is_empty() {
        let _ = fs::write(&dest, sub_data);
        return Some(dest);
    }

    None
}

pub fn play_source(
    item: &SourceItem,
    title: &str,
    ttid_opt: Option<&str>,
    ep_info: Option<(u32, u32)>,
) -> io::Result<Result<(), String>> {
    let sss_opt = find_sss_executable();

    // Subtitle fetching
    let sub_file = ttid_opt.and_then(|id| fetch_subtitles_for_id(id, ep_info));

    // Leave alternate screen & raw mode before launching player
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    let mut cmd: Command;

    match (&item.stream, sss_opt) {
        (StreamSource::Torrent { magnet, file_idx, .. }, Some(sss_path)) => {
            cmd = Command::new(sss_path);
            cmd.arg("magnet").arg(magnet).arg(title);
            if let Some(fidx) = file_idx {
                cmd.arg(fidx.to_string());
            }
        }
        (StreamSource::Torrent { magnet, .. }, None) => {
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            return Ok(Err(format!(
                "Backend streaming script not found. Please set SSS_PATH or place sss.txt in the application directory. Magnet: {magnet}"
            )));
        }
        (StreamSource::DirectUrl { url }, Some(sss_path)) => {
            cmd = Command::new(sss_path);
            cmd.arg("url").arg(url).arg(title);
        }
        (StreamSource::DirectUrl { url }, None) => {
            // Direct MPV fallback
            cmd = Command::new("mpv");
            cmd.arg(url);
            cmd.arg(format!("--force-media-title={title}"));
            cmd.arg("--keep-open=no");
            cmd.arg("--save-position-on-quit");
            cmd.arg("--cache=yes");
            cmd.arg("--demuxer-max-bytes=200M");
            cmd.arg("--demuxer-readahead-secs=40");
            if let Some(sub) = &sub_file {
                cmd.arg(format!("--sub-file={}", sub.display()));
            }
        }
        (StreamSource::SssPick { num }, Some(sss_path)) => {
            cmd = Command::new(sss_path);
            if let Some(id) = ttid_opt {
                cmd.arg("id").arg(id);
                if let Some((s, e)) = ep_info {
                    cmd.arg(s.to_string()).arg(e.to_string());
                }
                cmd.arg("-p").arg("mpv");
                cmd.env("SSS_PICK", num);
            } else {
                cmd.arg("url").arg(&item.title);
            }
        }
        (StreamSource::SssPick { .. }, None) => {
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            return Ok(Err("Backend streaming script (sss.txt) is required for Stremio/Torrentio resolver".to_string()));
        }
    }

    if let Some(sub) = &sub_file {
        cmd.env("SUBS_FILE", sub.to_string_lossy().to_string());
    }

    if env::var("SSS_TORRENT_WAIT").is_err() {
        cmd.env("SSS_TORRENT_WAIT", "18");
    }

    let status = cmd.status();

    // Restore alternate screen & raw mode
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    match status {
        Ok(s) => {
            if s.success() {
                Ok(Ok(()))
            } else {
                Ok(Err(format!("Player exited with status: {s}")))
            }
        }
        Err(e) => Ok(Err(format!("Failed to launch player: {e}"))),
    }
}
