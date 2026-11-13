//! tsss-tui - Modern, ergonomic interactive TUI for sss (Super Simple Streamer)
//!
//! Features:
//! - Full keyboard navigation (Arrows, vi keys j/k/h/l, Tab, 1-9 direct selection)
//! - Smart flow: Movies skip season/episode directly to sources; TV shows get an intuitive stepper
//! - Real-time loading indicators for network/source fetching
//! - Rich 2-column layout with Show Details & Source Inspector
//! - Source health badges (Seeders, Quality, Resolution, HDR, Size)
//! - Quality quick-filters (All, 4K, 1080p, 720p)
//! - Continuous playback & Binge-watching support: 'n' for next episode without restarting!

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::io;
use std::process::Command;
use std::time::Duration;

const SSS: &str = "/home/lk/webseries/sss.txt";
const IMDB_SUG: &str = "https://v3.sg.media-imdb.com/suggestion/x/{}.json";

#[derive(Deserialize, Debug, Clone)]
struct ImdbResult {
    id: String,
    q: Option<String>,
    qid: Option<String>,
    y: Option<serde_json::Value>,
    l: Option<String>,
    s: Option<String>,      // Starring cast
    rank: Option<u64>,     // Popularity rank
}

impl ImdbResult {
    fn is_series(&self) -> bool {
        let qid = (self.qid.clone().unwrap_or_default()
            + " "
            + &self.q.clone().unwrap_or_default())
        .to_lowercase();
        qid.contains("series") || qid.contains("tv")
    }

    fn year_str(&self) -> String {
        match &self.y {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => "----".to_string(),
        }
    }

    fn kind_label(&self) -> &'static str {
        if self.is_series() {
            "TV Series"
        } else {
            "Movie"
        }
    }
}

#[derive(Deserialize, Debug)]
struct ImdbResponse {
    d: Option<Vec<ImdbResult>>,
}

#[derive(Clone, Debug)]
struct SourceItem {
    num: String,
    provider: String,
    resolution: String,
    hdr_info: Option<String>,
    seeds: Option<u32>,
    size_str: String,
    title: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum QualityFilter {
    All,
    Res4k,
    Res1080p,
    Res720p,
}

impl QualityFilter {
    fn label(&self) -> &'static str {
        match self {
            QualityFilter::All => "All",
            QualityFilter::Res4k => "4K",
            QualityFilter::Res1080p => "1080p",
            QualityFilter::Res720p => "720p",
        }
    }

    fn matches(&self, item: &SourceItem) -> bool {
        match self {
            QualityFilter::All => true,
            QualityFilter::Res4k => item.resolution == "4K",
            QualityFilter::Res1080p => item.resolution == "1080p",
            QualityFilter::Res720p => item.resolution == "720p",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepperField {
    Season,
    Episode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Search,
    PickShow,
    SeasonEpisode,
    PickSource,
}

struct App {
    screen: Screen,
    query: String,
    results: Vec<ImdbResult>,
    show_idx: usize,
    show_state: ListState,
    season: u32,
    episode: u32,
    stepper_field: StepperField,
    sources: Vec<SourceItem>,
    src_idx: usize,
    src_state: ListState,
    quality_filter: QualityFilter,
    status_msg: Option<String>,
    error: Option<String>,
    is_loading: bool,
    loading_msg: String,
}

impl App {
    fn new() -> Self {
        App {
            screen: Screen::Search,
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
            status_msg: None,
            error: None,
            is_loading: false,
            loading_msg: String::new(),
        }
    }

    fn search(&mut self) {
        let q = self.query.trim().to_string();
        if q.is_empty() {
            self.status_msg = Some("Please type a search query first".to_string());
            return;
        }

        self.is_loading = true;
        self.loading_msg = format!("Searching IMDb for \"{}\"...", q);
    }

    fn execute_search(&mut self) {
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

    fn selected_show(&self) -> Option<&ImdbResult> {
        self.results.get(self.show_idx)
    }

    fn select_show_confirm(&mut self) {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };

        if show.is_series() {
            // For series, prompt for Season and Episode
            self.stepper_field = StepperField::Season;
            self.screen = Screen::SeasonEpisode;
            self.status_msg = None;
        } else {
            // For movies, bypass Season/Episode screen directly to sources!
            self.season = 1;
            self.episode = 1;
            self.is_loading = true;
            let title = show.l.as_deref().unwrap_or("Movie");
            self.loading_msg = format!("Fetching sources for {}...", title);
        }
    }

    fn prepare_load_sources(&mut self) {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };
        let title = show.l.as_deref().unwrap_or("Title");
        let is_tv = show.is_series();
        self.is_loading = true;
        if is_tv {
            self.loading_msg = format!(
                "Fetching sources for {} S{:02}E{:02}...",
                title, self.season, self.episode
            );
        } else {
            self.loading_msg = format!("Fetching sources for {}...", title);
        }
    }

    fn execute_load_sources(&mut self) {
        self.is_loading = false;
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return,
        };

        let ep_info = if show.is_series() {
            Some((self.season, self.episode))
        } else {
            None
        };

        match list_sources(&show.id, ep_info) {
            Ok(s) => {
                self.sources = s;
                self.quality_filter = QualityFilter::All;
                self.src_idx = 0;
                self.src_state = ListState::default();
                if !self.sources.is_empty() {
                    self.src_state.select(Some(0));
                }
                self.screen = Screen::PickSource;
                if self.sources.is_empty() {
                    self.status_msg = Some("No sources found for this title".to_string());
                } else {
                    self.status_msg = None;
                }
            }
            Err(e) => {
                self.error = Some(format!("Sources error: {e}"));
            }
        }
    }

    fn filtered_source_indices(&self) -> Vec<usize> {
        self.sources
            .iter()
            .enumerate()
            .filter(|(_, item)| self.quality_filter.matches(item))
            .map(|(i, _)| i)
            .collect()
    }

    fn play_selected(&mut self) -> io::Result<()> {
        let show = match self.selected_show() {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let filtered = self.filtered_source_indices();
        let actual_idx = match filtered.get(self.src_idx) {
            Some(&idx) => idx,
            None => return Ok(()),
        };
        let src = match self.sources.get(actual_idx) {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let mut env_vars = env::vars().collect::<std::collections::HashMap<_, _>>();
        env_vars.insert("SSS_PICK".to_string(), src.num.clone());
        env_vars.insert(
            "SSS_TORRENT_WAIT".to_string(),
            env::var("SSS_TORRENT_WAIT").unwrap_or_else(|_| "90".to_string()),
        );

        // Leave alternate screen & raw mode before launching player
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        let sss_path = env::var("SSS_PATH").unwrap_or_else(|_| SSS.to_string());
        let mut cmd = Command::new(&sss_path);
        cmd.arg("id").arg(&show.id);

        let is_tv = show.is_series();
        if is_tv {
            cmd.arg(self.season.to_string()).arg(self.episode.to_string());
        }
        cmd.arg("-p").arg("mpv").envs(env_vars);

        let status = cmd.status();

        // Restore alternate screen & raw mode
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        match status {
            Ok(s) => {
                if !s.success() {
                    self.error = Some(format!("Player exited with status: {s}"));
                } else {
                    let title = show.l.as_deref().unwrap_or("Title");
                    if is_tv {
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
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to launch player: {e}"));
            }
        }
        Ok(())
    }

    fn next_episode(&mut self) {
        let is_series = self.selected_show().map_or(false, |s| s.is_series());
        if !is_series {
            return;
        }
        self.episode = self.episode.saturating_add(1);
        self.prepare_load_sources();
    }
}

fn search_title(q: &str) -> Result<Vec<ImdbResult>, Box<dyn std::error::Error>> {
    let encoded = urlencoding::encode(q.trim());
    let url = IMDB_SUG.replace("{}", &encoded);
    let text = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(10))
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

fn parse_source_line(line: &str) -> Option<SourceItem> {
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
    let mut seeds = None;
    let mut size_str = "N/A".to_string();
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
        } else if p0_lower.contains("hdtv") {
            resolution = "HDTV".to_string();
        } else if p0_lower.contains("dvd") {
            resolution = "DVD".to_string();
        } else if p0_lower.contains("bdrip") {
            resolution = "BDRip".to_string();
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
            } else if pl == "hdr" || pl == "dv" || pl.contains("hdr") || pl.contains("dv") {
                hdr_info = Some(part.to_string());
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
        seeds,
        size_str,
        title,
    })
}

fn list_sources(
    ttid: &str,
    episode_info: Option<(u32, u32)>,
) -> Result<Vec<SourceItem>, String> {
    let sss_path = env::var("SSS_PATH").unwrap_or_else(|_| SSS.to_string());
    let mut cmd = Command::new(&sss_path);
    cmd.arg("id").arg(ttid);

    if let Some((season, episode)) = episode_info {
        cmd.arg(season.to_string()).arg(episode.to_string());
    }
    cmd.arg("--list").arg("--no-subs");

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute {sss_path}: {e}"))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut srcs = Vec::new();
    for line in text.lines() {
        if let Some(item) = parse_source_line(line) {
            srcs.push(item);
        }
    }
    Ok(srcs)
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ----------------------------------------------------------------------------
// UI Rendering
// ----------------------------------------------------------------------------

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Breadcrumbs
            Constraint::Min(0),    // Main workspace
            Constraint::Length(3), // Footer & Shortcut pills
        ])
        .split(size);

    // --- Header ---
    render_header(f, app, chunks[0]);

    // --- Main Screen ---
    match app.screen {
        Screen::Search => render_search_screen(f, app, chunks[1]),
        Screen::PickShow => render_pick_show_screen(f, app, chunks[1]),
        Screen::SeasonEpisode => render_season_episode_screen(f, app, chunks[1]),
        Screen::PickSource => render_pick_source_screen(f, app, chunks[1]),
    }

    // --- Footer ---
    render_footer(f, app, chunks[2]);

    // --- Status banner / toast ---
    if let Some(status) = &app.status_msg {
        if app.error.is_none() && !app.is_loading {
            let toast_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(chunks[0].height),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(size)[1];

            let toast = Paragraph::new(Line::from(vec![
                Span::styled(" ℹ ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
                Span::raw(" "),
                Span::styled(status, Style::default().fg(Color::Cyan).bold()),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(toast, toast_area);
        }
    }

    // --- Loading overlay ---
    if app.is_loading {
        let area = centered_rect(50, 25, size);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(" Please Wait ", Style::default().fg(Color::Yellow).bold()));

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(&app.loading_msg, Style::default().fg(Color::White).bold()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Connecting to media indexers...",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
    }

    // --- Error popup modal ---
    if let Some(e) = &app.error {
        let area = centered_rect(60, 30, size);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red))
            .title(Span::styled(" ⚠ Notice ", Style::default().fg(Color::Red).bold()));

        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(e.as_str(), Style::default().fg(Color::White))),
            Line::from(""),
            Line::from(Span::styled(
                "[ Press any key or Esc to dismiss ]",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        f.render_widget(p, area);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" 🎬 tsss ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::raw(" "),
    ];

    match app.screen {
        Screen::Search => {
            spans.push(Span::styled("Search", Style::default().fg(Color::White).bold()));
        }
        Screen::PickShow => {
            spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(" ❯ ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled("Pick Title", Style::default().fg(Color::Cyan).bold()));
            if let Some(show) = app.selected_show() {
                if let Some(title) = &show.l {
                    spans.push(Span::styled(format!(" ({title})"), Style::default().fg(Color::Gray)));
                }
            }
        }
        Screen::SeasonEpisode => {
            spans.push(Span::styled("Pick Title", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(" ❯ ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled("Season & Episode", Style::default().fg(Color::Cyan).bold()));
        }
        Screen::PickSource => {
            if let Some(show) = app.selected_show() {
                let title = show.l.as_deref().unwrap_or("Title");
                spans.push(Span::styled(title, Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(" ❯ ", Style::default().fg(Color::DarkGray)));
                if show.is_series() {
                    spans.push(Span::styled(
                        format!("S{:02}E{:02}", app.season, app.episode),
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    spans.push(Span::styled(" ❯ ", Style::default().fg(Color::DarkGray)));
                }
            }
            spans.push(Span::styled(
                format!("Sources ({})", app.sources.len()),
                Style::default().fg(Color::Cyan).bold(),
            ));
        }
    }

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let header_p = Paragraph::new(Line::from(spans)).block(header_block);
    f.render_widget(header_p, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let pills = match app.screen {
        Screen::Search => vec![
            ("Enter", "Search"),
            ("Type", "Query"),
            ("Esc", "Clear / Exit"),
        ],
        Screen::PickShow => vec![
            ("↑/↓, j/k", "Navigate"),
            ("1-9", "Quick Select"),
            ("Enter", "Confirm"),
            ("q / Esc", "Back to Search"),
        ],
        Screen::SeasonEpisode => vec![
            ("←/→", "Adjust Value"),
            ("Tab / ↑/↓", "Switch Field"),
            ("0-9", "Direct Type"),
            ("Enter", "Find Sources"),
            ("q / Esc", "Back to Shows"),
        ],
        Screen::PickSource => {
            let is_series = app.selected_show().map_or(false, |s| s.is_series());
            let mut p = vec![
                ("↑/↓, j/k", "Navigate"),
                ("1-9", "Select #"),
                ("Tab / f", "Filter Quality"),
                ("Enter", "Stream in MPV"),
            ];
            if is_series {
                p.push(("n", "Next Episode"));
            }
            p.push(("q / Esc", "Back"));
            p
        }
    };

    let mut spans = Vec::new();
    for (i, (key, desc)) in pills.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!(" [{key}] "),
            Style::default().fg(Color::Black).bg(Color::DarkGray).bold(),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*desc, Style::default().fg(Color::White)));
    }

    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let footer_p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(footer_block);
    f.render_widget(footer_p, area);
}

fn render_search_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Search input box
            Constraint::Length(1), // Spacing
            Constraint::Min(0),    // Tips / Info card
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" 🔍 Search Title ", Style::default().fg(Color::Cyan).bold()));

    let input_line = if app.query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Type movie or show name (e.g. Severance, Dune, Inception, Breaking Bad)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.query, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Cyan)),
        ])
    };

    let search_p = Paragraph::new(input_line).block(search_block);
    f.render_widget(search_p, chunks[1]);

    // Position terminal cursor right after input text
    f.set_cursor_position((
        chunks[1].x + 4 + (app.query.len() as u16).min(chunks[1].width.saturating_sub(6)),
        chunks[1].y + 1,
    ));

    // Info card below search input
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" Features & Quick Guide ", Style::default().fg(Color::Gray)));

    let tips = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  • Instant IMDb Search: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Find movies, TV series, mini-series and anime."),
        ]),
        Line::from(vec![
            Span::styled("  • Smart Flow: ", Style::default().fg(Color::Green).bold()),
            Span::raw("Movies stream immediately; TV series prompt for Season/Episode."),
        ]),
        Line::from(vec![
            Span::styled("  • Direct P2P Stream: ", Style::default().fg(Color::Yellow).bold()),
            Span::raw("Fast torrent streaming over HTTP directly to MPV with auto-subtitles."),
        ]),
        Line::from(vec![
            Span::styled("  • Binge-watching: ", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Press 'n' anytime to jump to the next episode without restarting."),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Popular: Severance, Breaking Bad, The Matrix, Shogun, Arcane, Interstellar",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let info_p = Paragraph::new(tips).block(info_block);
    f.render_widget(info_p, chunks[3]);
}

fn render_pick_show_screen(f: &mut Frame, app: &App, area: Rect) {
    if app.results.is_empty() {
        let empty_card = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("No titles found matching \"{}\"", app.query),
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "• Check spelling or try a broader keyword",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "• Press [Esc] or [q] to return to search",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Results "),
        );
        f.render_widget(empty_card, area);
        return;
    }

    // Split horizontally if width is sufficient (>= 80 cols)
    let is_wide = area.width >= 80;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        (cols[0], Some(cols[1]))
    } else {
        (area, None)
    };

    // Render Show List
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let year = r.year_str();
            let title = r.l.as_deref().unwrap_or("Unknown Title");
            let is_tv = r.is_series();

            let kind_badge = if is_tv {
                Span::styled(" [SERIES] ", Style::default().fg(Color::Cyan).bold())
            } else {
                Span::styled(" [MOVIE]  ", Style::default().fg(Color::Yellow).bold())
            };

            let num_span = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let title_span = Span::styled(title, Style::default().fg(Color::White).bold());
            let year_span = Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray));

            ListItem::new(Line::from(vec![num_span, kind_badge, title_span, year_span]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(" Titles ({} found) ", app.results.len()),
            Style::default().fg(Color::Cyan).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 45, 65))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.show_state.clone());

    // Render Show Details Card if wide screen
    if let Some(detail_box) = details_area {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Title Details ", Style::default().fg(Color::White).bold()));

        if let Some(show) = app.selected_show() {
            let title = show.l.as_deref().unwrap_or("Unknown");
            let year = show.year_str();
            let kind = show.kind_label();
            let id = &show.id;
            let stars = show.s.as_deref().unwrap_or("Cast details not available");
            let rank_str = show
                .rank
                .map(|r| format!("#{r} on IMDb"))
                .unwrap_or_else(|| "N/A".to_string());

            let action_hint = if show.is_series() {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to choose Season & Episode", Style::default().fg(Color::Cyan).bold()),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to fetch Movie Sources directly", Style::default().fg(Color::Yellow).bold()),
                ])
            };

            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(title, Style::default().fg(Color::White).bold().add_modifier(Modifier::UNDERLINED)),
                    Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Type:        ", Style::default().fg(Color::DarkGray)),
                    Span::styled(kind, Style::default().fg(if show.is_series() { Color::Cyan } else { Color::Yellow }).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Starring:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(stars, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("IMDb ID:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(id, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("Popularity:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(rank_str, Style::default().fg(Color::Green)),
                ]),
                Line::from(""),
                action_hint,
            ];

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("Select a title to view details").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

fn render_season_episode_screen(f: &mut Frame, app: &App, area: Rect) {
    let show = app.selected_show();
    let title = show.and_then(|s| s.l.as_deref()).unwrap_or("Series");
    let year = show.map(|s| s.year_str()).unwrap_or_else(|| "----".to_string());

    let card_area = centered_rect(65, 55, area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" Season & Episode Selection ", Style::default().fg(Color::Cyan).bold()));

    let inner = main_block.inner(card_area);
    f.render_widget(main_block, card_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title info
            Constraint::Length(3), // Season stepper
            Constraint::Length(1), // Spacing
            Constraint::Length(3), // Episode stepper
            Constraint::Length(2), // Preview badge
            Constraint::Min(0),    // Control hints
        ])
        .split(inner);

    // Show title header
    let title_line = Paragraph::new(Line::from(vec![
        Span::styled("Series: ", Style::default().fg(Color::DarkGray)),
        Span::styled(title, Style::default().fg(Color::White).bold()),
        Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title_line, rows[0]);

    // Season Stepper Box
    let is_season_active = app.stepper_field == StepperField::Season;
    let s_border_style = if is_season_active {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let s_prefix = if is_season_active { "▶ " } else { "  " };

    let s_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(s_border_style)
        .title(Span::styled(format!("{s_prefix}Season"), s_border_style));

    let s_content = Paragraph::new(Line::from(vec![
        Span::styled("◄  ", if is_season_active { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled(format!("{:02}", app.season), Style::default().fg(Color::White).bold()),
        Span::styled("  ►", if is_season_active { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled("   (← / → adjust, or type number)", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(s_box);
    f.render_widget(s_content, rows[1]);

    // Episode Stepper Box
    let is_ep_active = app.stepper_field == StepperField::Episode;
    let ep_border_style = if is_ep_active {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let ep_prefix = if is_ep_active { "▶ " } else { "  " };

    let ep_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(ep_border_style)
        .title(Span::styled(format!("{ep_prefix}Episode"), ep_border_style));

    let ep_content = Paragraph::new(Line::from(vec![
        Span::styled("◄  ", if is_ep_active { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled(format!("{:02}", app.episode), Style::default().fg(Color::White).bold()),
        Span::styled("  ►", if is_ep_active { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled("   (← / → adjust, or type number)", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(ep_box);
    f.render_widget(ep_content, rows[3]);

    // Preview badge
    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Season {}, Episode {}  (S{:02}E{:02})", app.season, app.episode, app.season, app.episode),
            Style::default().fg(Color::Yellow).bold(),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(preview, rows[4]);

    // Controls explanation
    let hints = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("[Tab / ↑ / ↓] ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch Field    "),
            Span::styled("[← / →] ", Style::default().fg(Color::Cyan)),
            Span::raw("Adjust    "),
            Span::styled("[0-9] ", Style::default().fg(Color::Cyan)),
            Span::raw("Type Number"),
        ]),
        Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(Color::Yellow).bold()),
            Span::raw("Fetch Sources     "),
            Span::styled("[q / Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Back"),
        ]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hints, rows[5]);
}

fn render_pick_source_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter bar
            Constraint::Min(0),    // Main sources list & detail inspector
        ])
        .split(area);

    // --- Quality Filter Bar ---
    let filters = [
        QualityFilter::All,
        QualityFilter::Res1080p,
        QualityFilter::Res4k,
        QualityFilter::Res720p,
    ];

    let mut filter_spans = vec![
        Span::styled(" Filter [Tab/f]: ", Style::default().fg(Color::DarkGray)),
    ];

    for qf in filters {
        let count = app.sources.iter().filter(|s| qf.matches(s)).count();
        let is_active = app.quality_filter == qf;
        let (bullet, style) = if is_active {
            ("● ", Style::default().fg(Color::Black).bg(Color::Cyan).bold())
        } else {
            ("○ ", Style::default().fg(Color::Gray))
        };
        let label = qf.label();
        filter_spans.push(Span::styled(format!(" [{bullet}{label} ({count})] "), style));
        filter_spans.push(Span::raw(" "));
    }

    let filter_bar = Paragraph::new(Line::from(filter_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(filter_bar, chunks[0]);

    // --- Filtered Sources List ---
    let filtered_indices = app.filtered_source_indices();

    if filtered_indices.is_empty() {
        let (heading, hint) = if app.sources.is_empty() {
            (
                "No sources found for this title or episode.",
                "Torrentio / indexers returned 0 streams. Press [Esc] to choose another title.",
            )
        } else {
            (
                "No sources match the selected quality filter.",
                "Press [Tab], [f], or [0] to switch filter back to All sources.",
            )
        };

        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(heading, Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(Span::styled(hint, Style::default().fg(Color::Cyan))),
            Line::from(""),
            Line::from(Span::styled("[Esc] Go back", Style::default().fg(Color::DarkGray))),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Sources "),
        );
        f.render_widget(empty_msg, chunks[1]);
        return;
    }

    let is_wide = chunks[1].width >= 85;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(chunks[1]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[1], None)
    };

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &actual_idx)| {
            let item = &app.sources[actual_idx];
            let num_span = Span::styled(format!("{:2}. ", display_idx + 1), Style::default().fg(Color::DarkGray));

            let res_style = match item.resolution.as_str() {
                "4K" => Style::default().fg(Color::Magenta).bold(),
                "1080p" => Style::default().fg(Color::Green).bold(),
                "720p" => Style::default().fg(Color::Cyan),
                _ => Style::default().fg(Color::DarkGray),
            };
            let res_span = Span::styled(format!("[{:5}] ", item.resolution), res_style);

            let seeds_span = match item.seeds {
                Some(s) if s >= 100 => {
                    Span::styled(format!("▲ {:4} seeds ", s), Style::default().fg(Color::Green))
                }
                Some(s) if s >= 20 => {
                    Span::styled(format!("▲ {:4} seeds ", s), Style::default().fg(Color::Yellow))
                }
                Some(s) => Span::styled(format!("▲ {:4} seeds ", s), Style::default().fg(Color::Red)),
                None => Span::styled("            ", Style::default().fg(Color::DarkGray)),
            };

            let size_span = Span::styled(format!("{:8} ", item.size_str), Style::default().fg(Color::Cyan));
            let title_span = Span::styled(&item.title, Style::default().fg(Color::White));

            ListItem::new(Line::from(vec![
                num_span, res_span, seeds_span, size_span, title_span,
            ]))
        })
        .collect();

    let title_prefix = app.selected_show().and_then(|s| s.l.as_deref()).unwrap_or("Sources");
    let ep_prefix = if app.selected_show().map_or(false, |s| s.is_series()) {
        format!(" S{:02}E{:02}", app.season, app.episode)
    } else {
        String::new()
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(" {title_prefix}{ep_prefix} - Sources ({}) ", filtered_indices.len()),
            Style::default().fg(Color::Cyan).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 45, 65))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.src_state.clone());

    // --- Source Details Inspector ---
    if let Some(detail_box) = details_area {
        let actual_idx = filtered_indices.get(app.src_idx).copied();
        let selected_item = actual_idx.and_then(|idx| app.sources.get(idx));

        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Stream Inspector ", Style::default().fg(Color::White).bold()));

        if let Some(item) = selected_item {
            let seeds_text = item
                .seeds
                .map(|s| {
                    if s >= 100 {
                        format!("{s} seeders (Excellent speed)")
                    } else if s >= 20 {
                        format!("{s} seeders (Good speed)")
                    } else {
                        format!("{s} seeders (May buffer)")
                    }
                })
                .unwrap_or_else(|| "Unknown peer count".to_string());

            let quality_text = if let Some(hdr) = &item.hdr_info {
                format!("{} ({hdr})", item.resolution)
            } else {
                item.resolution.clone()
            };

            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Resolution: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(quality_text, Style::default().fg(Color::Green).bold()),
                ]),
                Line::from(vec![
                    Span::styled("P2P Health: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(seeds_text, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("File Size:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.size_str, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Provider:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.provider, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Release Title:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&item.title, Style::default().fg(Color::White).bold())),
                Line::from(""),
                Line::from(Span::styled("Subtitles: Auto-attached via OpenSubtitles", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("Player:    MPV / HW Acceleration", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled(
                    "▶ Press [Enter] to stream directly",
                    Style::default().fg(Color::Cyan).bold(),
                )),
            ];

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("No source selected").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

// ----------------------------------------------------------------------------
// Main Loop & Event Handling
// ----------------------------------------------------------------------------

fn main() -> io::Result<()> {
    let mut app = App::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut exit = false;

    while !exit {
        terminal.draw(|f| ui(f, &app))?;

        // If a blocking action was triggered, run it after redrawing the loading screen
        if app.is_loading {
            match app.screen {
                Screen::Search => {
                    app.execute_search();
                }
                Screen::PickShow | Screen::SeasonEpisode => {
                    app.execute_load_sources();
                }
                Screen::PickSource => {
                    app.execute_load_sources();
                }
            }
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global Ctrl+C handler
            if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
                break;
            }

            // Dismiss active error popup with any key
            if app.error.is_some() {
                app.error = None;
                continue;
            }

            match app.screen {
                Screen::Search => match key.code {
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.query.clear();
                    }
                    KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let trimmed = app.query.trim_end();
                        if let Some(pos) = trimmed.rfind(' ') {
                            app.query.truncate(pos);
                        } else {
                            app.query.clear();
                        }
                    }
                    KeyCode::Esc => {
                        if !app.query.is_empty() {
                            app.query.clear();
                        } else {
                            exit = true;
                        }
                    }
                    KeyCode::Enter => {
                        app.search();
                    }
                    KeyCode::Backspace => {
                        app.query.pop();
                    }
                    KeyCode::Char(c) => {
                        app.query.push(c);
                    }
                    _ => {}
                },

                Screen::PickShow => {
                    let total = app.results.len();
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if total > 0 {
                                app.show_idx = app.show_idx.saturating_sub(1);
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if total > 0 && app.show_idx + 1 < total {
                                app.show_idx += 1;
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        KeyCode::PageUp => {
                            if total > 0 {
                                app.show_idx = app.show_idx.saturating_sub(5);
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        KeyCode::PageDown => {
                            if total > 0 {
                                app.show_idx = (app.show_idx + 5).min(total - 1);
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            if total > 0 {
                                app.show_idx = 0;
                                app.show_state.select(Some(0));
                            }
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            if total > 0 {
                                app.show_idx = total - 1;
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        // Quick 1-9 selection
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let digit_idx = (c.to_digit(10).unwrap_or(1) as usize) - 1;
                            if digit_idx < total {
                                app.show_idx = digit_idx;
                                app.show_state.select(Some(app.show_idx));
                            }
                        }
                        KeyCode::Enter => {
                            if total > 0 {
                                app.select_show_confirm();
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                            app.screen = Screen::Search;
                            app.status_msg = None;
                        }
                        _ => {}
                    }
                }

                Screen::SeasonEpisode => match key.code {
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                        app.stepper_field = match app.stepper_field {
                            StepperField::Season => StepperField::Episode,
                            StepperField::Episode => StepperField::Season,
                        };
                    }
                    KeyCode::Left | KeyCode::Char('-') => match app.stepper_field {
                        StepperField::Season => {
                            app.season = app.season.saturating_sub(1).max(1);
                        }
                        StepperField::Episode => {
                            app.episode = app.episode.saturating_sub(1).max(1);
                        }
                    },
                    KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                        match app.stepper_field {
                            StepperField::Season => {
                                app.season = (app.season + 1).min(99);
                            }
                            StepperField::Episode => {
                                app.episode = (app.episode + 1).min(999);
                            }
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let digit = c.to_digit(10).unwrap_or(0);
                        match app.stepper_field {
                            StepperField::Season => {
                                let new_val = if app.season > 9 { digit } else { app.season * 10 + digit };
                                app.season = new_val.clamp(1, 99);
                            }
                            StepperField::Episode => {
                                let new_val = if app.episode > 99 { digit } else { app.episode * 10 + digit };
                                app.episode = new_val.clamp(1, 999);
                            }
                        }
                    }
                    KeyCode::Backspace => match app.stepper_field {
                        StepperField::Season => {
                            app.season = (app.season / 10).max(1);
                        }
                        StepperField::Episode => {
                            app.episode = (app.episode / 10).max(1);
                        }
                    },
                    KeyCode::Enter => {
                        app.prepare_load_sources();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.screen = Screen::PickShow;
                        app.status_msg = None;
                    }
                    _ => {}
                },

                Screen::PickSource => {
                    let filtered_indices = app.filtered_source_indices();
                    let total = filtered_indices.len();

                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if total > 0 {
                                app.src_idx = app.src_idx.saturating_sub(1);
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if total > 0 && app.src_idx + 1 < total {
                                app.src_idx += 1;
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        KeyCode::PageUp => {
                            if total > 0 {
                                app.src_idx = app.src_idx.saturating_sub(5);
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        KeyCode::PageDown => {
                            if total > 0 {
                                app.src_idx = (app.src_idx + 5).min(total - 1);
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            if total > 0 {
                                app.src_idx = 0;
                                app.src_state.select(Some(0));
                            }
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            if total > 0 {
                                app.src_idx = total - 1;
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        // Filter cycling: Tab, BackTab, f, F
                        KeyCode::Tab | KeyCode::Char('f') => {
                            app.quality_filter = match app.quality_filter {
                                QualityFilter::All => QualityFilter::Res1080p,
                                QualityFilter::Res1080p => QualityFilter::Res4k,
                                QualityFilter::Res4k => QualityFilter::Res720p,
                                QualityFilter::Res720p => QualityFilter::All,
                            };
                            app.src_idx = 0;
                            app.src_state.select(Some(0));
                        }
                        KeyCode::BackTab | KeyCode::Char('F') => {
                            app.quality_filter = match app.quality_filter {
                                QualityFilter::All => QualityFilter::Res720p,
                                QualityFilter::Res720p => QualityFilter::Res4k,
                                QualityFilter::Res4k => QualityFilter::Res1080p,
                                QualityFilter::Res1080p => QualityFilter::All,
                            };
                            app.src_idx = 0;
                            app.src_state.select(Some(0));
                        }
                        // Reset filter to All: 0 or 'a'
                        KeyCode::Char('0') | KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.quality_filter = QualityFilter::All;
                            app.src_idx = 0;
                            app.src_state.select(Some(0));
                        }
                        // Direct item selection: 1-9
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let digit_idx = (c.to_digit(10).unwrap_or(1) as usize) - 1;
                            if digit_idx < total {
                                app.src_idx = digit_idx;
                                app.src_state.select(Some(app.src_idx));
                            }
                        }
                        // Binge-watching: next episode shortcut
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            app.next_episode();
                        }
                        KeyCode::Enter => {
                            if total > 0 {
                                if let Err(e) = app.play_selected() {
                                    app.error = Some(e.to_string());
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            let is_series = app.selected_show().map_or(false, |s| s.is_series());
                            if is_series {
                                app.screen = Screen::SeasonEpisode;
                            } else {
                                app.screen = Screen::PickShow;
                            }
                            app.status_msg = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_line_dots_and_quality() {
        let line = "  1.   Torrentio 1080p | seed:980   |   1.86 GB | The.Matrix.1999.1080p.BrRip.x264.YIFY.mp4";
        let parsed = parse_source_line(line).expect("Should parse successfully");
        assert_eq!(parsed.num, "1");
        assert_eq!(parsed.resolution, "1080p");
        assert_eq!(parsed.seeds, Some(980));
        assert_eq!(parsed.size_str, "1.86 GB");
        assert_eq!(parsed.title, "The.Matrix.1999.1080p.BrRip.x264.YIFY.mp4");
    }

    #[test]
    fn test_parse_source_line_4k_hdr() {
        let line = "  3.   Torrentio 4k DV | HDR | seed:352   |  18.45 GB | Матрица.1999.WEB-DL.2160p.mkv";
        let parsed = parse_source_line(line).expect("Should parse successfully");
        assert_eq!(parsed.num, "3");
        assert_eq!(parsed.resolution, "4K");
        assert_eq!(parsed.seeds, Some(352));
        assert_eq!(parsed.size_str, "18.45 GB");
        assert_eq!(parsed.hdr_info.as_deref(), Some("HDR"));
        assert_eq!(parsed.title, "Матрица.1999.WEB-DL.2160p.mkv");
    }

    #[test]
    fn test_is_series_detection() {
        let movie = ImdbResult {
            id: "tt0133093".to_string(),
            q: Some("feature".to_string()),
            qid: Some("movie".to_string()),
            y: Some(serde_json::json!(1999)),
            l: Some("The Matrix".to_string()),
            s: None,
            rank: None,
        };
        assert!(!movie.is_series());
        assert_eq!(movie.kind_label(), "Movie");
        assert_eq!(movie.year_str(), "1999");

        let tv = ImdbResult {
            id: "tt0903747".to_string(),
            q: Some("TV series".to_string()),
            qid: Some("tvSeries".to_string()),
            y: Some(serde_json::json!(2008)),
            l: Some("Breaking Bad".to_string()),
            s: None,
            rank: None,
        };
        assert!(tv.is_series());
        assert_eq!(tv.kind_label(), "TV Series");
        assert_eq!(tv.year_str(), "2008");
    }

    #[test]
    fn test_quality_filter() {
        let item_4k = SourceItem {
            num: "1".to_string(),
            provider: "Torrentio".to_string(),
            resolution: "4K".to_string(),
            hdr_info: None,
            seeds: Some(100),
            size_str: "10 GB".to_string(),
            title: "Test 4K".to_string(),
        };
        let item_1080p = SourceItem {
            num: "2".to_string(),
            provider: "Torrentio".to_string(),
            resolution: "1080p".to_string(),
            hdr_info: None,
            seeds: Some(50),
            size_str: "2 GB".to_string(),
            title: "Test 1080p".to_string(),
        };

        assert!(QualityFilter::All.matches(&item_4k));
        assert!(QualityFilter::All.matches(&item_1080p));
        assert!(QualityFilter::Res4k.matches(&item_4k));
        assert!(!QualityFilter::Res4k.matches(&item_1080p));
        assert!(!QualityFilter::Res1080p.matches(&item_4k));
        assert!(QualityFilter::Res1080p.matches(&item_1080p));
    }

    #[test]
    fn test_live_search_title() {
        let results = search_title("Inception").expect("Search should succeed");
        assert!(!results.is_empty(), "Should return at least one result for Inception");
        let first = &results[0];
        assert_eq!(first.l.as_deref(), Some("Inception"));
        assert!(!first.is_series());
    }
}
