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
    Seeders,
    Quality,
    Size,
}

impl SortOrder {
    pub fn label(&self) -> &'static str {
        match self {
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
