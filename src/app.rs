use crate::models::{
    AppMode, ArchiveFile, ArchiveItem, CartoonItem, HistoryEntry, ImdbResult, IptvChannel, KitsuAnime, ProviderFilter,
    QualityFilter, Screen, SortOrder, SourceItem, StepperField, StreamSource,
};
use crate::player;
use crate::sources::{
    anime::{fetch_all_anime_sources, search_anime},
    archive::{fetch_archive_files, search_archive},
    cartoon::{fetch_all_cartoon_sources, search_cartoons},
    fetch_all_streaming_sources,
    iptv::load_or_fetch_channels,
    search_title,
    torrentio::make_magnet,
};
use ratatui::widgets::ListState;
use std::io;

pub struct App {
    pub mode: AppMode,
    pub screen: Screen,
    pub show_help: bool,
    pub spinner_tick: usize,

    // --- Movies & Series State ---
    pub query: String,
    pub results: Vec<ImdbResult>,
    pub show_idx: usize,
    pub show_state: ListState,
    pub season: u32,
    pub episode: u32,
    pub stepper_field: StepperField,
    pub sources: Vec<SourceItem>,
    pub src_idx: usize,
    pub src_state: ListState,
    pub quality_filter: QualityFilter,
    pub provider_filter: ProviderFilter,
    pub sort_order: SortOrder,
    pub hide_foreign_dubs: bool,

    // --- Anime State ---
    pub anime_query: String,
    pub anime_results: Vec<KitsuAnime>,
    pub anime_idx: usize,
    pub anime_state: ListState,
    pub anime_episode: u32,
    pub anime_sources: Vec<SourceItem>,
    pub anime_src_idx: usize,
    pub anime_src_state: ListState,
    pub anime_quality_filter: QualityFilter,
    pub anime_provider_filter: ProviderFilter,
    pub anime_sort_order: SortOrder,

    // --- Cartoons State ---
    pub cartoon_query: String,
    pub cartoon_results: Vec<CartoonItem>,
    pub cartoon_idx: usize,
    pub cartoon_state: ListState,
    pub cartoon_season: u32,
    pub cartoon_episode: u32,
    pub cartoon_stepper_field: StepperField,
    pub cartoon_sources: Vec<SourceItem>,
    pub cartoon_src_idx: usize,
    pub cartoon_src_state: ListState,
    pub cartoon_quality_filter: QualityFilter,
    pub cartoon_provider_filter: ProviderFilter,
    pub cartoon_sort_order: SortOrder,

    // --- Live TV (IPTV) State ---
    pub iptv_channels: Vec<IptvChannel>,
    pub iptv_query: String,
    pub iptv_category: String,
    pub iptv_idx: usize,
    pub iptv_state: ListState,

    // --- Archive.org State ---
    pub archive_query: String,
    pub archive_results: Vec<ArchiveItem>,
    pub archive_idx: usize,
    pub archive_state: ListState,
    pub archive_files: Vec<ArchiveFile>,
    pub archive_file_idx: usize,
    pub archive_file_state: ListState,

    // --- Direct Stream State ---
    pub direct_url_input: String,

    // --- Watch History & Continue Watching ---
    pub history: Vec<HistoryEntry>,
    pub history_idx: usize,
    pub history_state: ListState,

    // --- Status & Feedback ---
    pub status_msg: Option<String>,
    pub error: Option<String>,
    pub is_loading: bool,
    pub loading_msg: String,
}

impl App {
    pub fn new() -> Self {
        let history = player::load_history();
        let mut history_state = ListState::default();
        if !history.is_empty() {
            history_state.select(Some(0));
        }

        App {
            mode: AppMode::MoviesAndSeries,
            screen: Screen::Search,
            show_help: false,
            spinner_tick: 0,

            query: String::new(),
            results: Vec::new(),
            show_idx: 0,
            show_state: ListState::default(),
            season: 1,
            episode: 1,
            stepper_field: StepperField::Season,
            sources: Vec::new(),
            src_idx: 0,
            src_state: ListState::default(),
            quality_filter: QualityFilter::All,
            provider_filter: ProviderFilter::All,
            sort_order: SortOrder::Recommended,
            hide_foreign_dubs: true,

            anime_query: String::new(),
            anime_results: Vec::new(),
            anime_idx: 0,
            anime_state: ListState::default(),
            anime_episode: 1,
            anime_sources: Vec::new(),
            anime_src_idx: 0,
            anime_src_state: ListState::default(),
            anime_quality_filter: QualityFilter::All,
            anime_provider_filter: ProviderFilter::All,
            anime_sort_order: SortOrder::Recommended,

            cartoon_query: String::new(),
            cartoon_results: Vec::new(),
            cartoon_idx: 0,
            cartoon_state: ListState::default(),
            cartoon_season: 1,
            cartoon_episode: 1,
            cartoon_stepper_field: StepperField::Season,
            cartoon_sources: Vec::new(),
            cartoon_src_idx: 0,
            cartoon_src_state: ListState::default(),
            cartoon_quality_filter: QualityFilter::All,
            cartoon_provider_filter: ProviderFilter::All,
            cartoon_sort_order: SortOrder::Recommended,

            iptv_channels: Vec::new(),
            iptv_query: String::new(),
            iptv_category: "All".to_string(),
            iptv_idx: 0,
            iptv_state: ListState::default(),

            archive_query: String::new(),
            archive_results: Vec::new(),
            archive_idx: 0,
            archive_state: ListState::default(),
            archive_files: Vec::new(),
            archive_file_idx: 0,
            archive_file_state: ListState::default(),

            direct_url_input: String::new(),

            history,
            history_idx: 0,
            history_state,

            status_msg: None,
            error: None,
            is_loading: false,
            loading_msg: String::new(),
        }
    }

    pub fn switch_mode(&mut self, new_mode: AppMode) {
        self.mode = new_mode;
        self.status_msg = None;
        self.error = None;
        self.is_loading = false;

        match new_mode {
            AppMode::MoviesAndSeries => {
                if self.results.is_empty() {
                    self.screen = Screen::Search;
                } else if !self.sources.is_empty() {
                    self.screen = Screen::PickSource;
                } else {
                    self.screen = Screen::PickShow;
                }
            }
            AppMode::Anime => {
                if self.anime_results.is_empty() && self.anime_sources.is_empty() {
                    self.screen = Screen::AnimeSearch;
                    self.is_loading = true;
                    self.loading_msg = "Fetching trending & popular anime...".to_string();
                } else if !self.anime_sources.is_empty() {
                    self.screen = Screen::AnimeSources;
                } else {
                    self.screen = Screen::AnimeSearch;
                }
            }
            AppMode::Cartoons => {
                if self.cartoon_results.is_empty() && self.cartoon_sources.is_empty() {
                    self.screen = Screen::CartoonSearch;
                    self.is_loading = true;
                    self.loading_msg = "Loading popular cartoons & animated shows...".to_string();
                } else if !self.cartoon_sources.is_empty() {
                    self.screen = Screen::CartoonSources;
                } else {
                    self.screen = Screen::CartoonSearch;
                }
            }
            AppMode::LiveTv => {
                self.screen = Screen::IptvList;
                if self.iptv_channels.is_empty() {
                    self.is_loading = true;
                    self.loading_msg = "Loading global live TV channel directory...".to_string();
                }
            }
            AppMode::Archive => {
                if self.archive_files.is_empty() {
                    self.screen = Screen::ArchiveSearch;
                } else {
                    self.screen = Screen::ArchiveFiles;
                }
            }
            AppMode::DirectStream => {
                self.screen = Screen::DirectInput;
            }
            AppMode::History => {
                self.history = player::load_history();
                self.history_idx = 0;
                self.history_state = ListState::default();
                if !self.history.is_empty() {
                    self.history_state.select(Some(0));
                }
                self.screen = Screen::History;
                self.status_msg = Some("Watch History & Continue Watching".to_string());
            }
        }
    }

    // ------------------------------------------------------------------------
    // Movies & TV Actions
    // ------------------------------------------------------------------------

    pub fn search(&mut self) {
        let q = self.query.trim().to_string();
        if q.is_empty() {
            self.status_msg = Some("Please type a search query first".to_string());
            return;
        }

        self.is_loading = true;
        self.loading_msg = format!("Searching IMDb for \"{q}\"...");
    }

    pub fn execute_search(&mut self) {
        let q = self.query.trim().to_string();
        self.is_loading = false;
        self.results.clear();
        self.show_idx = 0;
        self.show_state = ListState::default();

        match search_title(&q) {
            Ok(r) => {
                self.results = r;
                if !self.results.is_empty() {
                    self.show_state.select(Some(0));
                }
                self.screen = Screen::PickShow;
                self.status_msg = None;
            }
            Err(e) => {
                self.error = Some(format!("Search error: {e}"));
            }
        }
    }

    pub fn selected_show(&self) -> Option<&ImdbResult> {
        self.results.get(self.show_idx)
    }

    pub fn select_show_confirm(&mut self) {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };

        if show.is_series() {
            self.stepper_field = StepperField::Season;
            self.screen = Screen::SeasonEpisode;
            self.status_msg = None;
        } else {
            self.season = 1;
            self.episode = 1;
            self.prepare_load_sources();
        }
    }

    pub fn prepare_load_sources(&mut self) {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };
        let title = show.l.as_deref().unwrap_or("Title");
        let is_tv = show.is_series();
        self.is_loading = true;
        if is_tv {
            self.loading_msg = format!(
                "Searching multi-source streams for {} S{:02}E{:02}...",
                title, self.season, self.episode
            );
        } else {
            self.loading_msg = format!("Searching multi-source streams for {}...", title);
        }
    }

    pub fn execute_load_sources(&mut self) {
        self.is_loading = false;
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };

        let title = show.l.as_deref().unwrap_or("Title");
        let year = show.year_str();
        let is_series = show.is_series();

        let s = fetch_all_streaming_sources(
            &show.id,
            title,
            &year,
            is_series,
            self.season,
            self.episode,
        );

        self.sources = s;
        self.quality_filter = QualityFilter::All;
        self.provider_filter = ProviderFilter::All;
        self.sort_order = SortOrder::Recommended;
        self.src_idx = 0;
        self.src_state = ListState::default();

        if !self.sources.is_empty() {
            self.src_state.select(Some(0));
            self.status_msg = Some(format!(
                "Found {} stream sources across active providers",
                self.sources.len()
            ));
        } else {
            self.status_msg = Some("No sources found for this title across indexers".to_string());
        }

        self.screen = Screen::PickSource;
    }

    pub fn filtered_source_indices(&self) -> Vec<usize> {
        let has_non_dub = self.sources.iter().any(|s| !s.is_foreign_dub_only());
        let filter_dubs = self.hide_foreign_dubs && has_non_dub;

        let mut indices: Vec<usize> = self
            .sources
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if filter_dubs && item.is_foreign_dub_only() {
                    return false;
                }
                self.quality_filter.matches(item) && self.provider_filter.matches(item)
            })
            .map(|(i, _)| i)
            .collect();

        // Sort indices according to selected sort order
        match self.sort_order {
            SortOrder::Recommended => {
                indices.sort_by(|&a, &b| {
                    let score_b = self.sources[b].smart_score();
                    let score_a = self.sources[a].smart_score();
                    score_b.cmp(&score_a)
                });
            }
            SortOrder::Seeders => {
                indices.sort_by(|&a, &b| {
                    let seeds_b = self.sources[b].seeds.unwrap_or(0);
                    let seeds_a = self.sources[a].seeds.unwrap_or(0);
                    seeds_b.cmp(&seeds_a)
                });
            }
            SortOrder::Quality => {
                let rank = |res: &str| match res {
                    "4K" => 4,
                    "1080p" => 3,
                    "720p" => 2,
                    "480p" => 1,
                    _ => 0,
                };
                indices.sort_by(|&a, &b| {
                    let q_b = rank(&self.sources[b].resolution);
                    let q_a = rank(&self.sources[a].resolution);
                    q_b.cmp(&q_a)
                });
            }
            SortOrder::Size => {
                indices.sort_by(|&a, &b| {
                    let s_b = self.sources[b].size_bytes.unwrap_or(0);
                    let s_a = self.sources[a].size_bytes.unwrap_or(0);
                    s_b.cmp(&s_a)
                });
            }
        }

        indices
    }

    pub fn play_source_with_failover(&mut self, start_rank: usize) -> io::Result<()> {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let filtered = self.filtered_source_indices();
        if filtered.is_empty() {
            self.error = Some("No viable stream sources found for this title".to_string());
            return Ok(());
        }

        let ep_info = if show.is_series() {
            Some((self.season, self.episode))
        } else {
            None
        };
        let title = show.l.as_deref().unwrap_or("Title");

        let mut last_err = String::new();
        let max_attempts = 3.min(filtered.len().saturating_sub(start_rank).max(1));

        for attempt in 0..max_attempts {
            let rank = start_rank + attempt;
            let actual_idx = match filtered.get(rank) {
                Some(&idx) => idx,
                None => break,
            };
            let src = match self.sources.get(actual_idx) {
                Some(s) => s.clone(),
                None => break,
            };

            self.src_idx = rank;
            self.src_state.select(Some(rank));

            if attempt > 0 {
                eprintln!(
                    "\r\n\x1b[1;33m[tsss-tui Failover]\x1b[0m Stream candidate #{} failed/stalled. Auto-failing over to #{} ({} - {})...",
                    rank,
                    rank + 1,
                    src.provider,
                    src.resolution
                );
                std::thread::sleep(std::time::Duration::from_millis(1200));
            }

            self.record_history(
                "Movies & Series",
                title,
                Some(show.id.clone()),
                Some(show.year_str()),
                if show.is_series() { Some(self.season) } else { None },
                if show.is_series() { Some(self.episode) } else { None },
                &src.provider,
                &src.resolution,
                src.stream.clone(),
            );

            match player::play_source(&src, title, Some(&show.id), ep_info)? {
                Ok(()) => {
                    self.error = None;
                    if show.is_series() {
                        self.status_msg = Some(format!(
                            "Playback finished: {} S{:02}E{:02}. Press [N] for Next Episode!",
                            title, self.season, self.episode
                        ));
                    } else {
                        self.status_msg = Some(format!(
                            "Playback finished: {}. [Enter] to replay, [Esc] to go back.",
                            title
                        ));
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    if attempt + 1 < max_attempts {
                        self.status_msg = Some(format!(
                            "Stream candidate stalled, auto-failing over to candidate #{}...",
                            rank + 2
                        ));
                    }
                }
            }
        }

        self.error = Some(format!("All stream attempts failed: {last_err}"));
        Ok(())
    }

    pub fn play_selected_source(&mut self) -> io::Result<()> {
        self.play_source_with_failover(self.src_idx)
    }

    pub fn smart_play_current_title(&mut self) -> io::Result<()> {
        if self.sources.is_empty() {
            self.prepare_load_sources();
            self.execute_load_sources();
        }
        self.sort_order = SortOrder::Recommended;
        self.src_idx = 0;
        self.src_state.select(Some(0));
        self.play_source_with_failover(0)
    }

    pub fn next_episode(&mut self) {
        let is_series = self.selected_show().is_some_and(|s| s.is_series());
        if !is_series {
            return;
        }
        self.episode = self.episode.saturating_add(1);
        self.prepare_load_sources();
    }

    pub fn prev_episode(&mut self) {
        let is_series = self.selected_show().is_some_and(|s| s.is_series());
        if !is_series {
            return;
        }
        self.episode = self.episode.saturating_sub(1).max(1);
        self.prepare_load_sources();
    }

    // ------------------------------------------------------------------------
    // Live TV (IPTV) Actions
    // ------------------------------------------------------------------------

    pub fn execute_load_iptv(&mut self) {
        self.is_loading = false;
        match load_or_fetch_channels() {
            Ok(channels) => {
                self.iptv_channels = channels;
                self.iptv_idx = 0;
                self.iptv_state.select(Some(0));
                self.status_msg = Some(format!(
                    "Loaded {} live channels across 180+ categories",
                    self.iptv_channels.len()
                ));
            }
            Err(e) => {
                self.error = Some(format!("IPTV error: {e}"));
            }
        }
    }

    pub fn filtered_iptv_channels(&self) -> Vec<&IptvChannel> {
        let q = self.iptv_query.trim().to_lowercase();
        let cat = self.iptv_category.as_str();

        self.iptv_channels
            .iter()
            .filter(|ch| {
                let cat_match = if cat == "All" {
                    true
                } else {
                    ch.group.to_lowercase().contains(&cat.to_lowercase())
                };

                if !cat_match {
                    return false;
                }

                if q.is_empty() {
                    true
                } else {
                    ch.name.to_lowercase().contains(&q)
                        || ch.group.to_lowercase().contains(&q)
                }
            })
            .collect()
    }

    pub fn play_selected_iptv(&mut self) -> io::Result<()> {
        let filtered = self.filtered_iptv_channels();
        let ch = match filtered.get(self.iptv_idx) {
            Some(&c) => c.clone(),
            None => return Ok(()),
        };

        let dummy_item = SourceItem {
            num: "1".to_string(),
            provider: "IPTV".to_string(),
            resolution: ch.resolution.clone(),
            hdr_info: None,
            codec: None,
            seeds: None,
            size_bytes: None,
            size_str: "Live".to_string(),
            title: ch.name.clone(),
            stream: StreamSource::DirectUrl { url: ch.url },
        };

        self.record_history(
            "Live TV",
            &ch.name,
            None,
            None,
            None,
            None,
            "IPTV",
            &ch.resolution,
            dummy_item.stream.clone(),
        );

        match player::play_source(&dummy_item, &ch.name, None, None)? {
            Ok(()) => {
                self.status_msg = Some(format!("Live stream finished: {}", ch.name));
            }
            Err(e) => {
                self.error = Some(e);
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Internet Archive Actions
    // ------------------------------------------------------------------------

    pub fn search_archive(&mut self) {
        let q = self.archive_query.trim().to_string();
        if q.is_empty() {
            self.status_msg = Some("Please type a search query for Internet Archive".to_string());
            return;
        }

        self.is_loading = true;
        self.loading_msg = format!("Searching Internet Archive for \"{q}\"...");
    }

    pub fn execute_search_archive(&mut self) {
        let q = self.archive_query.trim().to_string();
        self.is_loading = false;
        self.archive_results.clear();
        self.archive_idx = 0;
        self.archive_state = ListState::default();

        match search_archive(&q) {
            Ok(results) => {
                self.archive_results = results;
                if !self.archive_results.is_empty() {
                    self.archive_state.select(Some(0));
                    self.status_msg = Some(format!(
                        "Found {} public domain titles on Archive.org",
                        self.archive_results.len()
                    ));
                } else {
                    self.status_msg = Some("No films found matching keyword on Archive.org".to_string());
                }
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn select_archive_item_confirm(&mut self) {
        let item = match self.archive_results.get(self.archive_idx) {
            Some(i) => i.clone(),
            None => return,
        };

        self.is_loading = true;
        self.loading_msg = format!("Fetching video stream files for {}...", item.title);
    }

    pub fn execute_load_archive_files(&mut self) {
        self.is_loading = false;
        let item = match self.archive_results.get(self.archive_idx) {
            Some(i) => i.clone(),
            None => return,
        };

        match fetch_archive_files(&item.identifier) {
            Ok(files) => {
                self.archive_files = files;
                self.archive_file_idx = 0;
                self.archive_file_state = ListState::default();
                if !self.archive_files.is_empty() {
                    self.archive_file_state.select(Some(0));
                    self.screen = Screen::ArchiveFiles;
                } else {
                    self.error = Some(format!("No playable video streams found in item {}", item.identifier));
                }
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    pub fn play_selected_archive_file(&mut self) -> io::Result<()> {
        let file = match self.archive_files.get(self.archive_file_idx) {
            Some(f) => f.clone(),
            None => return Ok(()),
        };

        let title = self
            .archive_results
            .get(self.archive_idx)
            .map(|i| i.title.clone())
            .unwrap_or_else(|| "Archive Movie".to_string());

        let dummy_item = SourceItem {
            num: "1".to_string(),
            provider: "Archive.org".to_string(),
            resolution: "Archive".to_string(),
            hdr_info: None,
            codec: None,
            seeds: None,
            size_bytes: Some(file.size_bytes),
            size_str: file.size_str,
            title: file.name.clone(),
            stream: StreamSource::DirectUrl { url: file.url },
        };

        self.record_history(
            "Archive",
            &title,
            None,
            None,
            None,
            None,
            "Archive.org",
            "Archive",
            dummy_item.stream.clone(),
        );

        match player::play_source(&dummy_item, &title, None, None)? {
            Ok(()) => {
                self.status_msg = Some(format!("Playback finished: {title}"));
            }
            Err(e) => {
                self.error = Some(e);
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Direct Stream Actions
    // ------------------------------------------------------------------------

    pub fn play_direct_input(&mut self) -> io::Result<()> {
        let input = self.direct_url_input.trim().to_string();
        if input.is_empty() {
            self.status_msg = Some("Please paste or type a magnet link or stream URL first".to_string());
            return Ok(());
        }

        let stream_source = if input.starts_with("magnet:") {
            let info_hash = input
                .split("xt=urn:btih:")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .unwrap_or("")
                .to_string();
            StreamSource::Torrent {
                magnet: input.clone(),
                info_hash,
                file_idx: None,
            }
        } else if input.len() == 40 && input.chars().all(|c| c.is_ascii_hexdigit()) {
            let magnet = make_magnet(&input, "Direct Stream", &[]);
            StreamSource::Torrent {
                magnet,
                info_hash: input.clone(),
                file_idx: None,
            }
        } else {
            StreamSource::DirectUrl { url: input.clone() }
        };

        let dummy_item = SourceItem {
            num: "1".to_string(),
            provider: "Direct".to_string(),
            resolution: "Direct".to_string(),
            hdr_info: None,
            codec: None,
            seeds: None,
            size_bytes: None,
            size_str: "Direct".to_string(),
            title: "Direct Stream".to_string(),
            stream: stream_source.clone(),
        };

        self.record_history(
            "Direct",
            &input,
            None,
            None,
            None,
            None,
            "Direct Stream",
            "Auto",
            stream_source,
        );

        match player::play_source(&dummy_item, "Direct Stream", None, None)? {
            Ok(()) => {
                self.status_msg = Some("Direct playback finished".to_string());
            }
            Err(e) => {
                self.error = Some(e);
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Anime Actions
    // ------------------------------------------------------------------------

    pub fn search_anime(&mut self) {
        let q = self.anime_query.trim().to_string();
        self.is_loading = true;
        if q.is_empty() {
            self.loading_msg = "Fetching trending & popular anime...".to_string();
        } else {
            self.loading_msg = format!("Searching anime for \"{q}\"...");
        }
    }

    pub fn execute_search_anime(&mut self) {
        let q = self.anime_query.trim().to_string();
        self.is_loading = false;
        self.anime_results.clear();
        self.anime_idx = 0;
        self.anime_state = ListState::default();

        match search_anime(&q) {
            Ok(results) => {
                self.anime_results = results;
                if !self.anime_results.is_empty() {
                    self.anime_state.select(Some(0));
                    if q.is_empty() {
                        self.status_msg = Some(format!(
                            "Loaded {} popular anime titles from Kitsu",
                            self.anime_results.len()
                        ));
                    } else {
                        self.status_msg = Some(format!(
                            "Found {} anime titles matching \"{q}\"",
                            self.anime_results.len()
                        ));
                    }
                } else {
                    self.status_msg = Some("No anime titles found matching keyword".to_string());
                }
                self.screen = Screen::AnimeSearch;
            }
            Err(e) => {
                self.error = Some(format!("Anime search error: {e}"));
            }
        }
    }

    pub fn selected_anime(&self) -> Option<&KitsuAnime> {
        self.anime_results.get(self.anime_idx)
    }

    pub fn select_anime_confirm(&mut self) {
        let anime = match self.selected_anime() {
            Some(a) => a.clone(),
            None => return,
        };

        if anime.is_movie() {
            self.anime_episode = 1;
            self.prepare_load_anime_sources();
        } else {
            self.anime_episode = 1;
            self.screen = Screen::AnimeEpisode;
            self.status_msg = None;
        }
    }

    pub fn prepare_load_anime_sources(&mut self) {
        let anime = match self.selected_anime() {
            Some(a) => a.clone(),
            None => return,
        };

        let title = anime.display_title();
        self.is_loading = true;
        if anime.is_movie() {
            self.loading_msg = format!("Searching anime multi-sources for {title} (Movie)...");
        } else {
            self.loading_msg = format!(
                "Searching anime multi-sources for {title} Ep {:02}...",
                self.anime_episode
            );
        }
    }

    pub fn execute_load_anime_sources(&mut self) {
        self.is_loading = false;
        let anime = match self.selected_anime() {
            Some(a) => a.clone(),
            None => return,
        };

        let sources = fetch_all_anime_sources(&anime, self.anime_episode);
        self.anime_sources = sources;
        self.anime_quality_filter = QualityFilter::All;
        self.anime_provider_filter = ProviderFilter::All;
        self.anime_sort_order = SortOrder::Recommended;
        self.anime_src_idx = 0;
        self.anime_src_state = ListState::default();

        if !self.anime_sources.is_empty() {
            self.anime_src_state.select(Some(0));
            self.status_msg = Some(format!(
                "Found {} stream sources across active anime indexers",
                self.anime_sources.len()
            ));
        } else {
            self.status_msg = Some("No sources found for this anime title/episode".to_string());
        }

        self.screen = Screen::AnimeSources;
    }

    pub fn filtered_anime_source_indices(&self) -> Vec<usize> {
        let has_non_dub = self.anime_sources.iter().any(|s| !s.is_foreign_dub_only());
        let filter_dubs = self.hide_foreign_dubs && has_non_dub;

        let mut indices: Vec<usize> = self
            .anime_sources
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if filter_dubs && item.is_foreign_dub_only() {
                    return false;
                }
                self.anime_quality_filter.matches(item) && self.anime_provider_filter.matches(item)
            })
            .map(|(i, _)| i)
            .collect();

        match self.anime_sort_order {
            SortOrder::Recommended => {
                indices.sort_by(|&a, &b| {
                    let score_b = self.anime_sources[b].smart_score();
                    let score_a = self.anime_sources[a].smart_score();
                    score_b.cmp(&score_a)
                });
            }
            SortOrder::Seeders => {
                indices.sort_by(|&a, &b| {
                    let seeds_b = self.anime_sources[b].seeds.unwrap_or(0);
                    let seeds_a = self.anime_sources[a].seeds.unwrap_or(0);
                    seeds_b.cmp(&seeds_a)
                });
            }
            SortOrder::Quality => {
                let rank = |res: &str| match res {
                    "4K" => 4,
                    "1080p" => 3,
                    "720p" => 2,
                    "480p" => 1,
                    _ => 0,
                };
                indices.sort_by(|&a, &b| {
                    let q_b = rank(&self.anime_sources[b].resolution);
                    let q_a = rank(&self.anime_sources[a].resolution);
                    q_b.cmp(&q_a)
                });
            }
            SortOrder::Size => {
                indices.sort_by(|&a, &b| {
                    let s_b = self.anime_sources[b].size_bytes.unwrap_or(0);
                    let s_a = self.anime_sources[a].size_bytes.unwrap_or(0);
                    s_b.cmp(&s_a)
                });
            }
        }

        indices
    }

    pub fn play_anime_source_with_failover(&mut self, start_rank: usize) -> io::Result<()> {
        let anime = match self.selected_anime() {
            Some(a) => a.clone(),
            None => return Ok(()),
        };

        let filtered = self.filtered_anime_source_indices();
        if filtered.is_empty() {
            self.error = Some("No anime sources found".to_string());
            return Ok(());
        }

        let title = anime.display_title().to_string();
        let is_movie = anime.is_movie();
        let ep_str = if is_movie {
            title.clone()
        } else {
            format!("{title} Episode {:02}", self.anime_episode)
        };

        let mut last_err = String::new();
        let max_attempts = 3.min(filtered.len().saturating_sub(start_rank).max(1));

        for attempt in 0..max_attempts {
            let rank = start_rank + attempt;
            let actual_idx = match filtered.get(rank) {
                Some(&idx) => idx,
                None => break,
            };
            let src = match self.anime_sources.get(actual_idx) {
                Some(s) => s.clone(),
                None => break,
            };

            self.anime_src_idx = rank;
            self.anime_src_state.select(Some(rank));

            if attempt > 0 {
                eprintln!(
                    "\r\n\x1b[1;33m[tsss-tui Failover]\x1b[0m Anime stream candidate #{} failed. Auto-failing over to #{} ({})...\r\n",
                    rank,
                    rank + 1,
                    src.resolution
                );
                std::thread::sleep(std::time::Duration::from_millis(1200));
            }

            self.record_history(
                "Anime",
                &title,
                None,
                Some(anime.year.clone()),
                None,
                if is_movie { None } else { Some(self.anime_episode) },
                &src.provider,
                &src.resolution,
                src.stream.clone(),
            );

            match player::play_source(&src, &ep_str, None, None)? {
                Ok(()) => {
                    self.error = None;
                    if is_movie {
                        self.status_msg = Some(format!(
                            "Playback finished: {title}. [Enter] to replay, [Esc] to go back."
                        ));
                    } else {
                        self.status_msg = Some(format!(
                            "Playback finished: {title} Episode {:02}. Press [N] for Next Episode!",
                            self.anime_episode
                        ));
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    if attempt + 1 < max_attempts {
                        self.status_msg = Some(format!(
                            "Anime candidate stalled, auto-failing over to candidate #{}...",
                            rank + 2
                        ));
                    }
                }
            }
        }

        self.error = Some(format!("All anime stream attempts failed: {last_err}"));
        Ok(())
    }

    pub fn play_selected_anime_source(&mut self) -> io::Result<()> {
        self.play_anime_source_with_failover(self.anime_src_idx)
    }

    pub fn smart_play_anime(&mut self) -> io::Result<()> {
        if self.anime_sources.is_empty() {
            self.prepare_load_anime_sources();
            self.execute_load_anime_sources();
        }
        self.anime_sort_order = SortOrder::Recommended;
        self.anime_src_idx = 0;
        self.anime_src_state.select(Some(0));
        self.play_anime_source_with_failover(0)
    }

    pub fn anime_next_episode(&mut self) {
        let is_movie = self.selected_anime().is_some_and(|a| a.is_movie());
        if is_movie {
            return;
        }
        let max_ep = self.selected_anime().and_then(|a| a.episode_count).unwrap_or(999);
        if self.anime_episode < max_ep {
            self.anime_episode = self.anime_episode.saturating_add(1);
            self.prepare_load_anime_sources();
        } else {
            self.status_msg = Some(format!("Already at final episode ({max_ep})"));
        }
    }

    pub fn anime_prev_episode(&mut self) {
        let is_movie = self.selected_anime().is_some_and(|a| a.is_movie());
        if is_movie {
            return;
        }
        if self.anime_episode > 1 {
            self.anime_episode = self.anime_episode.saturating_sub(1);
            self.prepare_load_anime_sources();
        }
    }

    // ------------------------------------------------------------------------
    // Cartoons Actions
    // ------------------------------------------------------------------------

    pub fn search_cartoons(&mut self) {
        let q = self.cartoon_query.trim().to_string();
        self.is_loading = true;
        if q.is_empty() {
            self.loading_msg = "Loading popular cartoons & animated shows...".to_string();
        } else {
            self.loading_msg = format!("Searching cartoons for \"{q}\"...");
        }
    }

    pub fn execute_search_cartoons(&mut self) {
        let q = self.cartoon_query.trim().to_string();
        self.is_loading = false;
        self.cartoon_results.clear();
        self.cartoon_idx = 0;
        self.cartoon_state = ListState::default();

        match search_cartoons(&q) {
            Ok(results) => {
                self.cartoon_results = results;
                if !self.cartoon_results.is_empty() {
                    self.cartoon_state.select(Some(0));
                    if q.is_empty() {
                        self.status_msg = Some(format!(
                            "Loaded {} popular animated shows & cartoons",
                            self.cartoon_results.len()
                        ));
                    } else {
                        self.status_msg = Some(format!(
                            "Found {} cartoons matching \"{q}\"",
                            self.cartoon_results.len()
                        ));
                    }
                } else {
                    self.status_msg = Some("No animated shows found matching query".to_string());
                }
                self.screen = Screen::CartoonSearch;
            }
            Err(e) => {
                self.error = Some(format!("Cartoon search error: {e}"));
            }
        }
    }

    pub fn selected_cartoon(&self) -> Option<&CartoonItem> {
        self.cartoon_results.get(self.cartoon_idx)
    }

    pub fn select_cartoon_confirm(&mut self) {
        let cartoon = match self.selected_cartoon() {
            Some(c) => c.clone(),
            None => return,
        };

        if cartoon.is_series {
            self.cartoon_season = 1;
            self.cartoon_episode = 1;
            self.cartoon_stepper_field = StepperField::Season;
            self.screen = Screen::CartoonSeasonEpisode;
            self.status_msg = None;
        } else {
            self.cartoon_season = 1;
            self.cartoon_episode = 1;
            self.prepare_load_cartoon_sources();
        }
    }

    pub fn prepare_load_cartoon_sources(&mut self) {
        let cartoon = match self.selected_cartoon() {
            Some(c) => c.clone(),
            None => return,
        };

        let title = &cartoon.title;
        self.is_loading = true;
        if cartoon.is_series {
            self.loading_msg = format!(
                "Searching multi-source streams for {title} S{:02}E{:02}...",
                self.cartoon_season, self.cartoon_episode
            );
        } else {
            self.loading_msg = format!("Searching multi-source streams for {title} (Animated Movie)...");
        }
    }

    pub fn execute_load_cartoon_sources(&mut self) {
        self.is_loading = false;
        let cartoon = match self.selected_cartoon() {
            Some(c) => c.clone(),
            None => return,
        };

        let sources = fetch_all_cartoon_sources(&cartoon, self.cartoon_season, self.cartoon_episode);
        self.cartoon_sources = sources;
        self.cartoon_quality_filter = QualityFilter::All;
        self.cartoon_provider_filter = ProviderFilter::All;
        self.cartoon_sort_order = SortOrder::Recommended;
        self.cartoon_src_idx = 0;
        self.cartoon_src_state = ListState::default();

        if !self.cartoon_sources.is_empty() {
            self.cartoon_src_state.select(Some(0));
            self.status_msg = Some(format!(
                "Found {} stream sources across active cartoon indexers",
                self.cartoon_sources.len()
            ));
        } else {
            self.status_msg = Some("No sources found for this cartoon title/episode".to_string());
        }

        self.screen = Screen::CartoonSources;
    }

    pub fn filtered_cartoon_source_indices(&self) -> Vec<usize> {
        let has_non_dub = self.cartoon_sources.iter().any(|s| !s.is_foreign_dub_only());
        let filter_dubs = self.hide_foreign_dubs && has_non_dub;

        let mut indices: Vec<usize> = self
            .cartoon_sources
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if filter_dubs && item.is_foreign_dub_only() {
                    return false;
                }
                self.cartoon_quality_filter.matches(item) && self.cartoon_provider_filter.matches(item)
            })
            .map(|(i, _)| i)
            .collect();

        match self.cartoon_sort_order {
            SortOrder::Recommended => {
                indices.sort_by(|&a, &b| {
                    let score_b = self.cartoon_sources[b].smart_score();
                    let score_a = self.cartoon_sources[a].smart_score();
                    score_b.cmp(&score_a)
                });
            }
            SortOrder::Seeders => {
                indices.sort_by(|&a, &b| {
                    let seeds_b = self.cartoon_sources[b].seeds.unwrap_or(0);
                    let seeds_a = self.cartoon_sources[a].seeds.unwrap_or(0);
                    seeds_b.cmp(&seeds_a)
                });
            }
            SortOrder::Quality => {
                let rank = |res: &str| match res {
                    "4K" => 4,
                    "1080p" => 3,
                    "720p" => 2,
                    "480p" => 1,
                    _ => 0,
                };
                indices.sort_by(|&a, &b| {
                    let q_b = rank(&self.cartoon_sources[b].resolution);
                    let q_a = rank(&self.cartoon_sources[a].resolution);
                    q_b.cmp(&q_a)
                });
            }
            SortOrder::Size => {
                indices.sort_by(|&a, &b| {
                    let s_b = self.cartoon_sources[b].size_bytes.unwrap_or(0);
                    let s_a = self.cartoon_sources[a].size_bytes.unwrap_or(0);
                    s_b.cmp(&s_a)
                });
            }
        }

        indices
    }

    pub fn play_cartoon_source_with_failover(&mut self, start_rank: usize) -> io::Result<()> {
        let cartoon = match self.selected_cartoon() {
            Some(c) => c.clone(),
            None => return Ok(()),
        };

        let filtered = self.filtered_cartoon_source_indices();
        if filtered.is_empty() {
            self.error = Some("No cartoon sources found".to_string());
            return Ok(());
        }

        let title = &cartoon.title;
        let is_series = cartoon.is_series;
        let ep_str = if is_series {
            format!("{title} S{:02}E{:02}", self.cartoon_season, self.cartoon_episode)
        } else {
            title.to_string()
        };
        let ep_info = if is_series {
            Some((self.cartoon_season, self.cartoon_episode))
        } else {
            None
        };

        let mut last_err = String::new();
        let max_attempts = 3.min(filtered.len().saturating_sub(start_rank).max(1));

        for attempt in 0..max_attempts {
            let rank = start_rank + attempt;
            let actual_idx = match filtered.get(rank) {
                Some(&idx) => idx,
                None => break,
            };
            let src = match self.cartoon_sources.get(actual_idx) {
                Some(s) => s.clone(),
                None => break,
            };

            self.cartoon_src_idx = rank;
            self.cartoon_src_state.select(Some(rank));

            if attempt > 0 {
                eprintln!(
                    "\r\n\x1b[1;33m[tsss-tui Failover]\x1b[0m Cartoon stream candidate #{} failed. Auto-failing over to #{} ({})...\r\n",
                    rank,
                    rank + 1,
                    src.resolution
                );
                std::thread::sleep(std::time::Duration::from_millis(1200));
            }

            self.record_history(
                "Cartoons",
                title,
                cartoon.imdb_id.clone(),
                Some(cartoon.year.clone()),
                if is_series { Some(self.cartoon_season) } else { None },
                if is_series { Some(self.cartoon_episode) } else { None },
                &src.provider,
                &src.resolution,
                src.stream.clone(),
            );

            match player::play_source(&src, &ep_str, cartoon.imdb_id.as_deref(), ep_info)? {
                Ok(()) => {
                    self.error = None;
                    if is_series {
                        self.status_msg = Some(format!(
                            "Playback finished: {title} S{:02}E{:02}. Press [N] for Next Episode!",
                            self.cartoon_season, self.cartoon_episode
                        ));
                    } else {
                        self.status_msg = Some(format!(
                            "Playback finished: {title}. [Enter] to replay, [Esc] to go back."
                        ));
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    if attempt + 1 < max_attempts {
                        self.status_msg = Some(format!(
                            "Cartoon candidate stalled, auto-failing over to candidate #{}...",
                            rank + 2
                        ));
                    }
                }
            }
        }

        self.error = Some(format!("All cartoon stream attempts failed: {last_err}"));
        Ok(())
    }

    pub fn play_selected_cartoon_source(&mut self) -> io::Result<()> {
        self.play_cartoon_source_with_failover(self.cartoon_src_idx)
    }

    pub fn smart_play_cartoon(&mut self) -> io::Result<()> {
        if self.cartoon_sources.is_empty() {
            self.prepare_load_cartoon_sources();
            self.execute_load_cartoon_sources();
        }
        self.cartoon_sort_order = SortOrder::Recommended;
        self.cartoon_src_idx = 0;
        self.cartoon_src_state.select(Some(0));
        self.play_cartoon_source_with_failover(0)
    }

    pub fn cartoon_next_episode(&mut self) {
        let is_series = self.selected_cartoon().is_some_and(|c| c.is_series);
        if !is_series {
            return;
        }
        self.cartoon_episode = self.cartoon_episode.saturating_add(1);
        self.prepare_load_cartoon_sources();
    }

    pub fn cartoon_prev_episode(&mut self) {
        let is_series = self.selected_cartoon().is_some_and(|c| c.is_series);
        if !is_series {
            return;
        }
        if self.cartoon_episode > 1 {
            self.cartoon_episode = self.cartoon_episode.saturating_sub(1);
            self.prepare_load_cartoon_sources();
        }
    }

    // ------------------------------------------------------------------------
    // Watch History & Continue Watching Actions
    // ------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn record_history(
        &mut self,
        mode: &str,
        title: &str,
        ttid: Option<String>,
        year: Option<String>,
        season: Option<u32>,
        episode: Option<u32>,
        provider: &str,
        resolution: &str,
        stream: StreamSource,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let id = format!("{mode}:{title}:{season:?}:{episode:?}");

        // Remove previous entry for the same title and mode to put fresh entry on top
        self.history.retain(|e| !(e.title.eq_ignore_ascii_case(title) && e.mode == mode));

        self.history.insert(
            0,
            HistoryEntry {
                id,
                mode: mode.to_string(),
                title: title.to_string(),
                ttid,
                year,
                season,
                episode,
                provider: provider.to_string(),
                resolution: resolution.to_string(),
                timestamp: now,
                stream,
            },
        );

        if self.history.len() > 100 {
            self.history.truncate(100);
        }

        let _ = player::save_history(&self.history);
    }

    pub fn delete_history_item(&mut self, idx: usize) {
        if idx < self.history.len() {
            self.history.remove(idx);
            let _ = player::save_history(&self.history);
            if self.history.is_empty() {
                self.history_idx = 0;
                self.history_state.select(None);
            } else {
                self.history_idx = self.history_idx.min(self.history.len() - 1);
                self.history_state.select(Some(self.history_idx));
            }
            self.status_msg = Some("Entry removed from watch history".to_string());
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_idx = 0;
        self.history_state.select(None);
        let _ = player::save_history(&self.history);
        self.status_msg = Some("Watch history cleared".to_string());
    }

    pub fn continue_watching_selected(&mut self, next_ep: bool) -> io::Result<()> {
        let entry = match self.history.get(self.history_idx) {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        match entry.mode.as_str() {
            "Movies & Series" => {
                if entry.season.is_some() || entry.episode.is_some() {
                    self.switch_mode(AppMode::MoviesAndSeries);
                    self.query = entry.title.clone();
                    let s = entry.season.unwrap_or(1);
                    let e = if next_ep {
                        entry.episode.unwrap_or(1).saturating_add(1)
                    } else {
                        entry.episode.unwrap_or(1)
                    };
                    self.season = s;
                    self.episode = e;
                    self.status_msg = Some(format!("Loading {} S{:02}E{:02}...", entry.title, s, e));
                    self.search();
                } else {
                    let dummy = SourceItem {
                        num: "1".to_string(),
                        provider: entry.provider.clone(),
                        resolution: entry.resolution.clone(),
                        hdr_info: None,
                        codec: None,
                        seeds: None,
                        size_bytes: None,
                        size_str: "Resume".to_string(),
                        title: entry.title.clone(),
                        stream: entry.stream.clone(),
                    };
                    let _ = player::play_source(&dummy, &entry.title, entry.ttid.as_deref(), None);
                }
            }
            "Anime" => {
                self.switch_mode(AppMode::Anime);
                self.anime_query = entry.title.clone();
                let ep = if next_ep {
                    entry.episode.unwrap_or(1).saturating_add(1)
                } else {
                    entry.episode.unwrap_or(1)
                };
                self.anime_episode = ep;
                self.status_msg = Some(format!("Loading {} Episode {:02}...", entry.title, ep));
                self.search_anime();
            }
            "Cartoons" => {
                self.switch_mode(AppMode::Cartoons);
                self.cartoon_query = entry.title.clone();
                let s = entry.season.unwrap_or(1);
                let ep = if next_ep {
                    entry.episode.unwrap_or(1).saturating_add(1)
                } else {
                    entry.episode.unwrap_or(1)
                };
                self.cartoon_season = s;
                self.cartoon_episode = ep;
                self.status_msg = Some(format!("Loading {} S{:02}E{:02}...", entry.title, s, ep));
                self.search_cartoons();
            }
            _ => {
                let dummy = SourceItem {
                    num: "1".to_string(),
                    provider: entry.provider.clone(),
                    resolution: entry.resolution.clone(),
                    hdr_info: None,
                    codec: None,
                    seeds: None,
                    size_bytes: None,
                    size_str: "Resume".to_string(),
                    title: entry.title.clone(),
                    stream: entry.stream.clone(),
                };
                let _ = player::play_source(&dummy, &entry.title, entry.ttid.as_deref(), None);
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------------
    // CLI Arguments Handler
    // ------------------------------------------------------------------------

    pub fn handle_cli_args(&mut self, args: &[String]) {
        let filtered_args: Vec<String> = args
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();

        if filtered_args.is_empty() {
            return;
        }

        let auto_play = args.iter().any(|a| a == "--play" || a == "-p" || a == "--auto");
        let first = &filtered_args[0].to_lowercase();

        match first.as_str() {
            "continue" | "history" => {
                self.switch_mode(AppMode::History);
            }
            "iptv" | "live" => {
                self.switch_mode(AppMode::LiveTv);
                if filtered_args.len() > 1 {
                    self.iptv_query = filtered_args[1..].join(" ");
                }
            }
            "archive" => {
                self.switch_mode(AppMode::Archive);
                if filtered_args.len() > 1 {
                    self.archive_query = filtered_args[1..].join(" ");
                    self.search_archive();
                }
            }
            "anime" => {
                self.switch_mode(AppMode::Anime);
                if filtered_args.len() > 1 {
                    let (title, _, ep) = parse_title_season_episode(&filtered_args[1..]);
                    self.anime_query = title;
                    if let Some(e) = ep {
                        self.anime_episode = e;
                    }
                    if let Ok(results) = crate::sources::anime::search_anime(&self.anime_query) {
                        if !results.is_empty() {
                            self.anime_results = results;
                            self.anime_idx = 0;
                            self.anime_state.select(Some(0));
                            self.prepare_load_anime_sources();
                            self.execute_load_anime_sources();
                            if auto_play && !self.anime_sources.is_empty() {
                                let _ = self.play_selected_anime_source();
                            }
                        }
                    }
                }
            }
            "cartoon" | "cartoons" => {
                self.switch_mode(AppMode::Cartoons);
                if filtered_args.len() > 1 {
                    let (title, s, ep) = parse_title_season_episode(&filtered_args[1..]);
                    self.cartoon_query = title;
                    if let Some(season) = s {
                        self.cartoon_season = season;
                    }
                    if let Some(episode) = ep {
                        self.cartoon_episode = episode;
                    }
                    if let Ok(results) = crate::sources::cartoon::search_cartoons(&self.cartoon_query) {
                        if !results.is_empty() {
                            self.cartoon_results = results;
                            self.cartoon_idx = 0;
                            self.cartoon_state.select(Some(0));
                            self.prepare_load_cartoon_sources();
                            self.execute_load_cartoon_sources();
                            if auto_play && !self.cartoon_sources.is_empty() {
                                let _ = self.play_selected_cartoon_source();
                            }
                        }
                    }
                }
            }
            "tv" | "series" => {
                self.switch_mode(AppMode::MoviesAndSeries);
                if filtered_args.len() > 1 {
                    let (title, s, ep) = parse_title_season_episode(&filtered_args[1..]);
                    self.query = title;
                    if let Some(season) = s {
                        self.season = season;
                    }
                    if let Some(episode) = ep {
                        self.episode = episode;
                    }
                    if let Ok(results) = crate::sources::search_title(&self.query) {
                        if !results.is_empty() {
                            self.results = results;
                            self.show_idx = 0;
                            self.show_state.select(Some(0));
                            self.prepare_load_sources();
                            self.execute_load_sources();
                            if auto_play && !self.sources.is_empty() {
                                let _ = self.play_selected_source();
                            }
                        }
                    }
                }
            }
            _ => {
                // General query, e.g. "The Mentalist" 1 8 or "The Mentalist Season 1 Episode 8"
                self.switch_mode(AppMode::MoviesAndSeries);
                let (title, s, ep) = parse_title_season_episode(&filtered_args);
                self.query = title;
                if let Some(season) = s {
                    self.season = season;
                }
                if let Some(episode) = ep {
                    self.episode = episode;
                }
                if let Ok(results) = crate::sources::search_title(&self.query) {
                    if !results.is_empty() {
                        self.results = results;
                        self.show_idx = 0;
                        self.show_state.select(Some(0));
                        let is_series = self.results[0].is_series();
                        if is_series || s.is_some() || ep.is_some() {
                            self.prepare_load_sources();
                            self.execute_load_sources();
                            if auto_play && !self.sources.is_empty() {
                                let _ = self.play_selected_source();
                            }
                        } else {
                            self.select_show_confirm();
                            self.execute_load_sources();
                            if auto_play && !self.sources.is_empty() {
                                let _ = self.play_selected_source();
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn parse_number(s: &str) -> Option<u32> {
    if let Ok(n) = s.parse::<u32>() {
        return Some(n);
    }
    match s.to_lowercase().as_str() {
        "one" | "first" => Some(1),
        "two" | "second" => Some(2),
        "three" | "third" => Some(3),
        "four" | "fourth" => Some(4),
        "five" | "fifth" => Some(5),
        "six" | "sixth" => Some(6),
        "seven" | "seventh" => Some(7),
        "eight" | "eighth" => Some(8),
        "nine" | "ninth" => Some(9),
        "ten" | "tenth" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        _ => None,
    }
}

pub fn parse_title_season_episode(args: &[String]) -> (String, Option<u32>, Option<u32>) {
    let joined = args.join(" ");
    let words: Vec<&str> = joined.split_whitespace().collect();
    if words.is_empty() {
        return (String::new(), None, None);
    }

    // 1. Check words for SxxExx like "S01E08" or "s1e8"
    for (idx, word) in words.iter().enumerate() {
        let w = word.to_lowercase();
        if w.starts_with('s') {
            if let Some(e_idx) = w.find('e') {
                let s_part = &w[1..e_idx];
                let e_part = &w[e_idx + 1..];
                if let (Some(s_num), Some(e_num)) = (parse_number(s_part), parse_number(e_part)) {
                    let title = words[..idx].join(" ");
                    if !title.is_empty() {
                        return (title, Some(s_num), Some(e_num));
                    }
                }
            }
        }
    }

    // 2. Check for "season X" and/or "episode Y" anywhere in words
    let mut title_words = Vec::new();
    let mut season_val = None;
    let mut episode_val = None;
    let mut i = 0;
    while i < words.len() {
        let w = words[i].to_lowercase();
        if (w == "season" || w == "s") && i + 1 < words.len() {
            if let Some(num) = parse_number(words[i + 1]) {
                season_val = Some(num);
                i += 2;
                continue;
            }
        }
        if (w == "episode" || w == "ep" || w == "e") && i + 1 < words.len() {
            if let Some(num) = parse_number(words[i + 1]) {
                episode_val = Some(num);
                i += 2;
                continue;
            }
        }
        title_words.push(words[i]);
        i += 1;
    }

    if season_val.is_some() || episode_val.is_some() {
        let title = title_words.join(" ");
        return (title, season_val, episode_val);
    }

    // 3. Check trailing numbers: [title..., season, episode]
    let len = words.len();
    if len >= 3 {
        if let (Some(s), Some(e)) = (parse_number(words[len - 2]), parse_number(words[len - 1])) {
            let title = words[..len - 2].join(" ");
            return (title, Some(s), Some(e));
        }
    }

    // 4. Check trailing single number: [title..., episode]
    if len >= 2 {
        if let Some(e) = parse_number(words[len - 1]) {
            let title = words[..len - 1].join(" ");
            return (title, None, Some(e));
        }
    }

    (words.join(" "), None, None)
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
