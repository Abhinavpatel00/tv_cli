use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ImdbResult {
    pub id: String,
    pub q: Option<String>,
    pub qid: Option<String>,
    pub y: Option<serde_json::Value>,
    pub l: Option<String>,
    pub s: Option<String>,      // Starring cast
    pub rank: Option<u64>,     // Popularity rank
}

impl ImdbResult {
    pub fn is_series(&self) -> bool {
        let qid = (self.qid.clone().unwrap_or_default()
            + " "
            + &self.q.clone().unwrap_or_default())
        .to_lowercase();
        qid.contains("series") || qid.contains("tv")
    }

    pub fn year_str(&self) -> String {
        match &self.y {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => "----".to_string(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        if self.is_series() {
            "TV Series"
        } else {
            "Movie"
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ImdbResponse {
    pub d: Option<Vec<ImdbResult>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamSource {
    Torrent {
        magnet: String,
        info_hash: String,
        file_idx: Option<u32>,
    },
    DirectUrl {
        url: String,
    },
    SssPick {
        num: String,
    },
}

#[derive(Clone, Debug)]
pub struct SourceItem {
    pub num: String,
    pub provider: String,
    pub resolution: String,
    pub hdr_info: Option<String>,
    pub codec: Option<String>,
    pub seeds: Option<u32>,
    pub size_bytes: Option<u64>,
    pub size_str: String,
    pub title: String,
    pub stream: StreamSource,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum StreamHealth {
    Stalled, // 0-4 seeds: high risk of stalling
    Slow,    // 5-14 seeds: low peers, may buffer slowly
    Fair,    // 15-39 seeds: stable swarm
    Good,    // 40-79 seeds: fast buffering
    Instant, // 80+ seeds: instant libtorrent buffer
}

impl StreamHealth {
    pub fn badge(&self) -> (&'static str, &'static str) {
        match self {
            StreamHealth::Instant => ("🟢 INSTANT", "Instant buffer (80+ seeds)"),
            StreamHealth::Good => ("🟢 FAST", "Fast swarm (40-79 seeds)"),
            StreamHealth::Fair => ("🟡 GOOD", "Stable peers (15-39 seeds)"),
            StreamHealth::Slow => ("🟠 SLOW", "Low peers (5-14 seeds)"),
            StreamHealth::Stalled => ("🔴 STALLED", "Risk of stall (<5 seeds)"),
        }
    }
}

impl SourceItem {
    pub fn health(&self) -> StreamHealth {
        match &self.stream {
            StreamSource::DirectUrl { .. } => StreamHealth::Instant,
            _ => match self.seeds {
                Some(s) if s >= 80 => StreamHealth::Instant,
                Some(s) if s >= 40 => StreamHealth::Good,
                Some(s) if s >= 15 => StreamHealth::Fair,
                Some(s) if s >= 5 => StreamHealth::Slow,
                _ => StreamHealth::Stalled,
            },
        }
    }

    pub fn is_foreign_dub_only(&self) -> bool {
        let lower = self.title.to_lowercase();

        // If it explicitly mentions English or multi/dual audio, it's not foreign-only
        let has_multi_or_eng = lower.contains("eng")
            || lower.contains("english")
            || lower.contains("multi")
            || lower.contains("dual")
            || lower.contains("en-");

        if has_multi_or_eng {
            return false;
        }

        let foreign_markers = [
            "rus", "russian", "dublado", "legenda", "castellano", "espanol",
            "french", "truefrench", "ita", "italian", "german", "deutsch",
            "hindi", "tamil", "telugu", "latino",
        ];

        for marker in &foreign_markers {
            if let Some(pos) = lower.find(marker) {
                let before = if pos == 0 { ' ' } else { lower[..pos].chars().last().unwrap_or(' ') };
                let after = lower[pos + marker.len()..].chars().next().unwrap_or(' ');
                let delim = |c: char| !c.is_alphanumeric();
                if delim(before) && delim(after) {
                    return true;
                }
            }
        }
        false
    }

    pub fn smart_score(&self) -> i64 {
        let mut score: i64 = 0;

        // 1. Swarm Health & Seeders
        match &self.stream {
            StreamSource::DirectUrl { .. } => {
                score += 550; // Dedicated HTTP/CDN server
            }
            _ => {
                if let Some(s) = self.seeds {
                    if s >= 200 {
                        score += 650;
                    } else if s >= 100 {
                        score += 550;
                    } else if s >= 50 {
                        score += 450;
                    } else if s >= 20 {
                        score += 350;
                    } else if s >= 10 {
                        score += 200;
                    } else if s >= 5 {
                        score += 100;
                    } else if s > 0 {
                        score -= 100;
                    } else {
                        score -= 400; // 0 seeds: dead swarm
                    }
                } else {
                    score += 50;
                }
            }
        }

        // 2. Resolution: 1080p and 720p provide optimal balance of fidelity & fast buffering
        match self.resolution.as_str() {
            "1080p" => score += 250,
            "720p" => score += 220,
            "4K" => score += 180,
            "480p" | "SD" => score += 80,
            _ => score += 50,
        }

        // 3. File Size Sweet Spot (optimal TV episode: 350MB - 3.5GB)
        if let Some(bytes) = self.size_bytes {
            let mb = bytes as f64 / (1024.0 * 1024.0);
            if (350.0..=3500.0).contains(&mb) {
                score += 200;
            } else if (200.0..=6000.0).contains(&mb) {
                score += 120;
            } else if mb > 12000.0 {
                score -= 120; // Full season packs or giant remuxes take longer to allocate
            } else if mb < 150.0 {
                score -= 150; // Likely sample or low quality
            }
        }

        // 4. Source & Codec Quality
        let lower = self.title.to_lowercase();
        if lower.contains("web-dl") || lower.contains("webrip") || lower.contains("bluray") {
            score += 120;
        } else if lower.contains("hdtv") {
            score += 80;
        }

        if lower.contains("x265") || lower.contains("hevc") || lower.contains("h.265") || lower.contains("10bit") {
            score += 90;
        } else if lower.contains("x264") || lower.contains("h.264") || lower.contains("avc") {
            score += 70;
        }

        // Avoid Camrips / Telesyncs
        if lower.contains("camrip") || lower.contains("telesync") || lower.contains("hdts") || lower.contains(".cam.") {
            score -= 600;
        }

        // 5. Heavy penalty for foreign dub only
        if self.is_foreign_dub_only() {
            score -= 500;
        }

        // 6. Bonus for multi/dual audio
        if lower.contains("multi") || lower.contains("dual") {
            score += 40;
        }

        score
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QualityFilter {
    All,
    Res4k,
    Res1080p,
    Res720p,
    Res480p,
}

impl QualityFilter {
    pub fn label(&self) -> &'static str {
        match self {
            QualityFilter::All => "All",
            QualityFilter::Res4k => "4K",
            QualityFilter::Res1080p => "1080p",
            QualityFilter::Res720p => "720p",
            QualityFilter::Res480p => "480p",
        }
    }

    pub fn matches(&self, item: &SourceItem) -> bool {
        match self {
            QualityFilter::All => true,
            QualityFilter::Res4k => item.resolution == "4K",
            QualityFilter::Res1080p => item.resolution == "1080p",
            QualityFilter::Res720p => item.resolution == "720p",
            QualityFilter::Res480p => item.resolution == "480p" || item.resolution == "SD",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProviderFilter {
    All,
    Torrentio,
    NyaaSi,
    ThePirateBay,
    TorrentsCsv,
    ArchiveOrg,
}

impl ProviderFilter {
    pub fn label(&self) -> &'static str {
        match self {
            ProviderFilter::All => "All Providers",
            ProviderFilter::Torrentio => "Torrentio",
            ProviderFilter::NyaaSi => "NyaaSi",
            ProviderFilter::ThePirateBay => "ThePirateBay",
            ProviderFilter::TorrentsCsv => "Torrents-CSV",
            ProviderFilter::ArchiveOrg => "Archive.org",
        }
    }

    pub fn matches(&self, item: &SourceItem) -> bool {
        match self {
            ProviderFilter::All => true,
            ProviderFilter::Torrentio => item.provider.contains("Torrentio"),
            ProviderFilter::NyaaSi => item.provider.contains("Nyaa"),
            ProviderFilter::ThePirateBay => {
                item.provider.contains("PirateBay")
                    || item.provider.contains("The Pirate Bay")
                    || item.provider.contains("TPB")
            }
            ProviderFilter::TorrentsCsv => {
                item.provider.contains("Torrents-CSV") || item.provider.contains("CSV")
            }
            ProviderFilter::ArchiveOrg => item.provider.contains("Archive"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortOrder {
    Recommended,
    Seeders,
    Quality,
    Size,
}

impl SortOrder {
    pub fn label(&self) -> &'static str {
        match self {
            SortOrder::Recommended => "★ Recommended",
            SortOrder::Seeders => "Seeders",
            SortOrder::Quality => "Quality",
            SortOrder::Size => "Size",
        }
    }
}

#[derive(Clone, Debug)]
pub struct IptvChannel {
    pub group: String,
    pub name: String,
    pub url: String,
    pub resolution: String,
}

#[derive(Clone, Debug)]
pub struct ArchiveItem {
    pub identifier: String,
    pub title: String,
    pub year: String,
    pub downloads: u64,
}

#[derive(Clone, Debug)]
pub struct ArchiveFile {
    pub name: String,
    pub size_str: String,
    pub size_bytes: u64,
    pub url: String,
    pub format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KitsuAnime {
    pub id: String,
    pub title: String,
    pub title_en: Option<String>,
    pub title_jp: Option<String>,
    pub synopsis: String,
    pub episode_count: Option<u32>,
    pub subtype: String, // "TV", "movie", "OVA", "special", "ONA"
    pub status: String,
    pub year: String,
    pub rating: Option<String>,
    pub age_rating_guide: Option<String>,
}

impl KitsuAnime {
    pub fn is_movie(&self) -> bool {
        self.subtype.eq_ignore_ascii_case("movie")
    }

    pub fn display_title(&self) -> &str {
        if let Some(en) = &self.title_en {
            if !en.is_empty() {
                return en.as_str();
            }
        }
        &self.title
    }

    pub fn subtype_label(&self) -> &'static str {
        match self.subtype.to_lowercase().as_str() {
            "movie" => "Movie",
            "tv" => "TV Series",
            "ova" => "OVA",
            "ona" => "ONA",
            "special" => "Special",
            _ => "Anime",
        }
    }

    pub fn status_label(&self) -> &'static str {
        match self.status.to_lowercase().as_str() {
            "finished" => "Finished",
            "current" => "Currently Airing",
            "upcoming" => "Upcoming",
            _ => "Released",
        }
    }

    pub fn episodes_display(&self) -> String {
        if let Some(c) = self.episode_count {
            format!("{c} eps")
        } else if self.status.eq_ignore_ascii_case("current") {
            "Airing".to_string()
        } else {
            "? eps".to_string()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CartoonItem {
    pub id: String,
    pub imdb_id: Option<String>,
    pub title: String,
    pub year: String,
    pub is_series: bool,
    pub genres: Vec<String>,
    pub network: Option<String>,
    pub status: String,
    pub summary: String,
    pub rating: Option<String>,
}

impl CartoonItem {
    pub fn display_network(&self) -> &str {
        self.network.as_deref().unwrap_or("Animation")
    }

    pub fn genres_str(&self) -> String {
        if self.genres.is_empty() {
            "Animation".to_string()
        } else {
            self.genres.join(", ")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    MoviesAndSeries,
    Anime,
    Cartoons,
    LiveTv,
    Archive,
    DirectStream,
    History,
}

impl AppMode {
    pub fn title(&self) -> &'static str {
        match self {
            AppMode::MoviesAndSeries => "Movies & Series",
            AppMode::Anime => "Anime",
            AppMode::Cartoons => "Cartoons & Animation",
            AppMode::LiveTv => "Live TV (IPTV)",
            AppMode::Archive => "Internet Archive",
            AppMode::DirectStream => "Direct Stream",
            AppMode::History => "Continue Watching",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepperField {
    Season,
    Episode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    // Movies & TV
    Search,
    PickShow,
    SeasonEpisode,
    PickSource,

    // Anime
    AnimeSearch,
    AnimeEpisode,
    AnimeSources,

    // Cartoons
    CartoonSearch,
    CartoonSeasonEpisode,
    CartoonSources,

    // Live TV
    IptvList,

    // Archive.org
    ArchiveSearch,
    ArchiveFiles,

    // Direct stream
    DirectInput,

    // Watch History / Continue Watching
    History,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub mode: String,
    pub title: String,
    pub ttid: Option<String>,
    pub year: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub provider: String,
    pub resolution: String,
    pub timestamp: u64,
    pub stream: StreamSource,
}

impl HistoryEntry {
    pub fn badge_label(&self) -> &'static str {
        match self.mode.as_str() {
            "Anime" => "ANIME",
            "Cartoons" => "CARTOON",
            "Movies & Series" => {
                if self.season.is_some() || self.episode.is_some() {
                    "SERIES"
                } else {
                    "MOVIE"
                }
            }
            "Live TV" => "IPTV",
            "Archive" => "ARCHIVE",
            _ => "DIRECT",
        }
    }

    pub fn progress_str(&self) -> String {
        match (self.season, self.episode) {
            (Some(s), Some(e)) => format!("S{:02}E{:02}", s, e),
            (None, Some(e)) => format!("EP {:02}", e),
            _ => "Feature / Stream".to_string(),
        }
    }

    pub fn next_episode_str(&self) -> Option<String> {
        match (self.season, self.episode) {
            (Some(s), Some(e)) => Some(format!("S{:02}E{:02}", s, e + 1)),
            (None, Some(e)) => Some(format!("EP {:02}", e + 1)),
            _ => None,
        }
    }

    pub fn time_ago_str(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(self.timestamp);

        let diff = now.saturating_sub(self.timestamp);
        if diff < 60 {
            "Just now".to_string()
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }
}
