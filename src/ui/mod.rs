use crate::app::App;
use crate::models::{
    AppMode, QualityFilter, Screen, StepperField, StreamSource,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn render_ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Mode Navigation
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
        Screen::AnimeSearch => render_anime_search_screen(f, app, chunks[1]),
        Screen::AnimeEpisode => render_anime_episode_screen(f, app, chunks[1]),
        Screen::AnimeSources => render_anime_sources_screen(f, app, chunks[1]),
        Screen::CartoonSearch => render_cartoon_search_screen(f, app, chunks[1]),
        Screen::CartoonSeasonEpisode => render_cartoon_season_episode_screen(f, app, chunks[1]),
        Screen::CartoonSources => render_cartoon_sources_screen(f, app, chunks[1]),
        Screen::IptvList => render_iptv_screen(f, app, chunks[1]),
        Screen::ArchiveSearch => render_archive_search_screen(f, app, chunks[1]),
        Screen::ArchiveFiles => render_archive_files_screen(f, app, chunks[1]),
        Screen::DirectInput => render_direct_input_screen(f, app, chunks[1]),
        Screen::History => render_history_screen(f, app, chunks[1]),
    }

    // --- Footer ---
    render_footer(f, app, chunks[2]);

    // --- Status banner / toast ---
    if let Some(status) = &app.status_msg {
        if app.error.is_none() && !app.is_loading && !app.show_help {
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
        let area = centered_rect(54, 28, size);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(" Fetching Streams ", Style::default().fg(Color::Yellow).bold()));

        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spin_char = spinner_frames[app.spinner_tick % spinner_frames.len()];

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!(" {spin_char} "), Style::default().fg(Color::Yellow).bold()),
                Span::styled(&app.loading_msg, Style::default().fg(Color::White).bold()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Aggregating: Torrentio • ThePirateBay • Torrents-CSV",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "[Esc] to cancel",
                Style::default().fg(Color::Gray).italic(),
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

    // --- Help modal overlay ---
    if app.show_help {
        render_help_modal(f, area_or_default(size));
    }
}

fn area_or_default(size: Rect) -> Rect {
    centered_rect(75, 75, size)
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let modes = [
        (AppMode::MoviesAndSeries, "F1 Movies/TV"),
        (AppMode::Anime, "F2 Anime"),
        (AppMode::Cartoons, "F3 Cartoons"),
        (AppMode::LiveTv, "F4 Live TV"),
        (AppMode::Archive, "F5 Archive.org"),
        (AppMode::DirectStream, "F6 Direct Stream"),
        (AppMode::History, "F7 History"),
    ];

    let mut spans = vec![
        Span::styled(" 🎬 tsss ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
        Span::raw("  "),
    ];

    for (m, label) in modes {
        let is_active = app.mode == m;
        let (style, prefix) = if is_active {
            (
                Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
                "● ",
            )
        } else {
            (
                Style::default().fg(Color::Gray),
                "○ ",
            )
        };
        spans.push(Span::styled(format!(" [{prefix}{label}] "), style));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::raw("  ❯ "));

    match app.screen {
        Screen::Search => {
            spans.push(Span::styled("Search Titles", Style::default().fg(Color::White).bold()));
        }
        Screen::PickShow => {
            spans.push(Span::styled("Pick Title", Style::default().fg(Color::Cyan).bold()));
            if let Some(show) = app.selected_show() {
                if let Some(title) = &show.l {
                    spans.push(Span::styled(format!(" ({title})"), Style::default().fg(Color::Gray)));
                }
            }
        }
        Screen::SeasonEpisode => {
            spans.push(Span::styled("Season & Episode", Style::default().fg(Color::Cyan).bold()));
        }
        Screen::PickSource => {
            if let Some(show) = app.selected_show() {
                let title = show.l.as_deref().unwrap_or("Title");
                spans.push(Span::styled(title, Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ❯ "));
                if show.is_series() {
                    spans.push(Span::styled(
                        format!("S{:02}E{:02}", app.season, app.episode),
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    spans.push(Span::raw(" ❯ "));
                }
            }
            spans.push(Span::styled(
                format!("Sources ({})", app.sources.len()),
                Style::default().fg(Color::Cyan).bold(),
            ));
        }
        Screen::AnimeSearch => {
            spans.push(Span::styled("Anime Catalog (Kitsu)", Style::default().fg(Color::Magenta).bold()));
            if let Some(anime) = app.selected_anime() {
                spans.push(Span::styled(format!(" ({})", anime.display_title()), Style::default().fg(Color::Gray)));
            }
        }
        Screen::AnimeEpisode => {
            if let Some(anime) = app.selected_anime() {
                spans.push(Span::styled(anime.display_title(), Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ❯ "));
            }
            spans.push(Span::styled(
                format!("Episode {:02}", app.anime_episode),
                Style::default().fg(Color::Magenta).bold(),
            ));
        }
        Screen::AnimeSources => {
            if let Some(anime) = app.selected_anime() {
                spans.push(Span::styled(anime.display_title(), Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ❯ "));
                if !anime.is_movie() {
                    spans.push(Span::styled(
                        format!("Ep {:02}", app.anime_episode),
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    spans.push(Span::raw(" ❯ "));
                }
            }
            spans.push(Span::styled(
                format!("Anime Sources ({})", app.anime_sources.len()),
                Style::default().fg(Color::Magenta).bold(),
            ));
        }
        Screen::CartoonSearch => {
            spans.push(Span::styled("Cartoons & Animation", Style::default().fg(Color::Yellow).bold()));
            if let Some(cartoon) = app.selected_cartoon() {
                spans.push(Span::styled(format!(" ({})", cartoon.title), Style::default().fg(Color::Gray)));
            }
        }
        Screen::CartoonSeasonEpisode => {
            if let Some(cartoon) = app.selected_cartoon() {
                spans.push(Span::styled(&cartoon.title, Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ❯ "));
            }
            spans.push(Span::styled(
                format!("S{:02}E{:02}", app.cartoon_season, app.cartoon_episode),
                Style::default().fg(Color::Yellow).bold(),
            ));
        }
        Screen::CartoonSources => {
            if let Some(cartoon) = app.selected_cartoon() {
                spans.push(Span::styled(&cartoon.title, Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ❯ "));
                if cartoon.is_series {
                    spans.push(Span::styled(
                        format!("S{:02}E{:02}", app.cartoon_season, app.cartoon_episode),
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    spans.push(Span::raw(" ❯ "));
                }
            }
            spans.push(Span::styled(
                format!("Cartoon Sources ({})", app.cartoon_sources.len()),
                Style::default().fg(Color::Yellow).bold(),
            ));
        }
        Screen::IptvList => {
            spans.push(Span::styled(
                format!("Live Channels ({})", app.iptv_channels.len()),
                Style::default().fg(Color::Green).bold(),
            ));
        }
        Screen::ArchiveSearch => {
            spans.push(Span::styled("Public Domain Movies", Style::default().fg(Color::Magenta).bold()));
        }
        Screen::ArchiveFiles => {
            spans.push(Span::styled("Select Video File", Style::default().fg(Color::Magenta).bold()));
        }
        Screen::DirectInput => {
            spans.push(Span::styled("Direct Magnet / URL", Style::default().fg(Color::White).bold()));
        }
        Screen::History => {
            spans.push(Span::styled(
                format!("Continue Watching ({})", app.history.len()),
                Style::default().fg(Color::Yellow).bold(),
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
    let mut pills = match app.screen {
        Screen::Search => vec![
            ("Enter", "Search"),
            ("F1-F6", "Switch Mode"),
            ("?", "Help"),
            ("Esc", "Exit"),
        ],
        Screen::PickShow => vec![
            ("↑/↓, j/k", "Navigate"),
            ("1-9", "Quick Select"),
            ("Enter", "Confirm"),
            ("q / Esc", "Back to Search"),
        ],
        Screen::SeasonEpisode => vec![
            ("Enter", "⚡ Instant Play"),
            ("v", "Browse Sources"),
            ("←/→", "Adjust Value"),
            ("Tab", "Switch Field"),
            ("q / Esc", "Back"),
        ],
        Screen::PickSource => {
            let is_series = app.selected_show().is_some_and(|s| s.is_series());
            let mut p = vec![
                ("↑/↓, j/k", "Navigate"),
                ("Enter", "Stream MPV"),
                ("s", "Sort (★)"),
                ("e", "Dubs"),
                ("Tab/f", "Quality"),
                ("p", "Provider"),
            ];
            if is_series {
                p.push(("n/p", "Next/Prev Ep"));
            }
            p.push(("q/Esc", "Back"));
            p
        }
        Screen::AnimeSearch => vec![
            ("Enter", "Confirm / Ep"),
            ("↑/↓, j/k", "Navigate"),
            ("1-9", "Quick Select"),
            ("Type", "Search"),
            ("F1-F6", "Switch Mode"),
            ("?", "Help"),
            ("Esc", "Clear"),
        ],
        Screen::AnimeEpisode => vec![
            ("Enter", "⚡ Instant Play"),
            ("v", "Browse Sources"),
            ("←/→, -/+", "Adjust Ep"),
            ("q / Esc", "Back"),
        ],
        Screen::AnimeSources => {
            let is_movie = app.selected_anime().is_some_and(|a| a.is_movie());
            let mut p = vec![
                ("↑/↓, j/k", "Navigate"),
                ("Enter", "Stream MPV"),
                ("s", "Sort (★)"),
                ("e", "Dubs"),
                ("Tab/f", "Quality"),
                ("p", "Provider"),
            ];
            if !is_movie {
                p.push(("n/P", "Next/Prev Ep"));
            }
            p.push(("q/Esc", "Back"));
            p
        }
        Screen::CartoonSearch => vec![
            ("Enter", "Confirm / S&E"),
            ("↑/↓, j/k", "Navigate"),
            ("1-9", "Quick Select"),
            ("Type", "Search"),
            ("F1-F6", "Switch Mode"),
            ("?", "Help"),
            ("Esc", "Clear"),
        ],
        Screen::CartoonSeasonEpisode => vec![
            ("Enter", "⚡ Instant Play"),
            ("v", "Browse Sources"),
            ("←/→", "Adjust Value"),
            ("Tab", "Switch Field"),
            ("q / Esc", "Back"),
        ],
        Screen::CartoonSources => {
            let is_series = app.selected_cartoon().is_some_and(|c| c.is_series);
            let mut p = vec![
                ("↑/↓, j/k", "Navigate"),
                ("Enter", "Stream MPV"),
                ("s", "Sort (★)"),
                ("e", "Dubs"),
                ("Tab/f", "Quality"),
                ("p", "Provider"),
            ];
            if is_series {
                p.push(("n/P", "Next/Prev Ep"));
            }
            p.push(("q/Esc", "Back"));
            p
        }
        Screen::IptvList => vec![
            ("↑/↓, j/k", "Navigate"),
            ("Tab / [/]", "Category"),
            ("Type", "Filter"),
            ("Enter", "Watch Live"),
            ("F1-F6", "Mode"),
        ],
        Screen::ArchiveSearch => vec![
            ("Enter", "Search"),
            ("↑/↓", "Navigate"),
            ("Enter", "Inspect Files"),
            ("F1-F6", "Mode"),
        ],
        Screen::ArchiveFiles => vec![
            ("↑/↓", "Select File"),
            ("Enter", "Stream MP4"),
            ("q / Esc", "Back"),
        ],
        Screen::DirectInput => vec![
            ("Enter", "Stream Magnet/URL"),
            ("Ctrl+U", "Clear"),
            ("F1-F7", "Mode"),
            ("Esc", "Back"),
        ],
        Screen::History => vec![
            ("↑/↓, j/k", "Navigate"),
            ("Enter", "Resume Current"),
            ("n", "Next Episode"),
            ("d", "Delete Entry"),
            ("c", "Clear All"),
            ("F1-F7", "Mode"),
            ("Esc", "Back"),
        ],
    };

    pills.push(("?", "Help Guide"));

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

// ----------------------------------------------------------------------------
// Screen Renderers
// ----------------------------------------------------------------------------

fn render_search_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Search input box
            Constraint::Length(1), // Spacing
            Constraint::Min(0),    // Features & Sources Guide
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" 🔍 Search Title (Movies & Series) ", Style::default().fg(Color::Cyan).bold()));

    let input_line = if app.query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Type movie or show name (e.g. Severance, Dune, Inception, Breaking Bad, Shogun)...",
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

    f.set_cursor_position((
        chunks[1].x + 4 + (app.query.len() as u16).min(chunks[1].width.saturating_sub(6)),
        chunks[1].y + 1,
    ));

    // Data Sources & Guide Card
    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" Active Streaming Data Sources ", Style::default().fg(Color::Yellow).bold()));

    let tips = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚡ Multi-Source Aggregation: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Simultaneously indexes torrents across multiple providers:"),
        ]),
        Line::from(vec![
            Span::styled("     • Torrentio: ", Style::default().fg(Color::Green).bold()),
            Span::raw("YTS, EZTV, 1337x, RARBG, TorrentGalaxy, MagnetDL, Knaben, Nyaa"),
        ]),
        Line::from(vec![
            Span::styled("     • The Pirate Bay (Apibay): ", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Direct live query for movies, anime, and series releases"),
        ]),
        Line::from(vec![
            Span::styled("     • Torrents-CSV: ", Style::default().fg(Color::Yellow).bold()),
            Span::raw("Decentralized open-source torrent search with 35M+ indexed records"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📺 Additional Streaming Modes: ", Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::from(vec![
            Span::styled("     • Press [F2]: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Live TV (IPTV) - 11,000+ live television channels (News, Sports, Movies)"),
        ]),
        Line::from(vec![
            Span::styled("     • Press [F3]: ", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Internet Archive - Public domain feature films, cult cinema & direct MP4 streams"),
        ]),
        Line::from(vec![
            Span::styled("     • Press [F4]: ", Style::default().fg(Color::White).bold()),
            Span::raw("Direct Stream - Stream any Magnet BTIH link or HLS .m3u8/.mp4 directly in MPV"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Shortcuts: [Tab] cycle filters  •  [n/p] Next/Previous episode  •  [?] Help cheat sheet",
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

    // Target badge
    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Season {}, Episode {}  (S{:02}E{:02})", app.season, app.episode, app.season, app.episode),
            Style::default().fg(Color::Yellow).bold(),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(preview, rows[4]);

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
            Span::styled("[Enter] ", Style::default().fg(Color::Green).bold()),
            Span::styled("⚡ Instant Play (Auto-Best)    ", Style::default().fg(Color::Green).bold()),
            Span::styled("[v] ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Browse Sources    ", Style::default().fg(Color::Yellow)),
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
            Constraint::Length(3), // Filter & Sort Bar
            Constraint::Min(0),    // Main sources list & inspector
        ])
        .split(area);

    // --- Filter & Sort Bar ---
    let q_filters = [
        QualityFilter::All,
        QualityFilter::Res4k,
        QualityFilter::Res1080p,
        QualityFilter::Res720p,
        QualityFilter::Res480p,
    ];

    let mut filter_spans = vec![
        Span::styled(" Quality [Tab/f]: ", Style::default().fg(Color::DarkGray)),
    ];

    for qf in q_filters {
        let count = app.sources.iter().filter(|s| qf.matches(s)).count();
        let is_active = app.quality_filter == qf;
        let (bullet, style) = if is_active {
            ("● ", Style::default().fg(Color::Black).bg(Color::Cyan).bold())
        } else {
            ("○ ", Style::default().fg(Color::Gray))
        };
        let label = qf.label();
        filter_spans.push(Span::styled(format!(" [{bullet}{label}:{count}] "), style));
        filter_spans.push(Span::raw(" "));
    }

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Provider [p]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.provider_filter.label()),
        Style::default().fg(Color::Yellow).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Sort [s]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.sort_order.label()),
        Style::default().fg(Color::Green).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Dubs [e]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        if app.hide_foreign_dubs { " [Hide] " } else { " [Show] " },
        if app.hide_foreign_dubs {
            Style::default().fg(Color::Green).bold()
        } else {
            Style::default().fg(Color::Yellow).bold()
        },
    ));

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
                "No sources found across active providers.",
                "Torrentio, The Pirate Bay, and Torrents-CSV returned 0 streams. Press [Esc] to choose another title.",
            )
        } else {
            (
                "No sources match the selected quality or provider filter.",
                "Press [f] to cycle quality or [p] to reset provider filter to All.",
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

            let recom_span = if display_idx == 0 {
                Span::styled("★ TOP ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::raw("      ")
            };

            let (h_tag, _) = item.health().badge();
            let h_span = match item.health() {
                crate::models::StreamHealth::Instant => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green).bold()),
                crate::models::StreamHealth::Good => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green)),
                crate::models::StreamHealth::Fair => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Yellow)),
                crate::models::StreamHealth::Slow => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Rgb(255, 165, 0))),
                crate::models::StreamHealth::Stalled => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Red)),
            };

            // Provider badge
            let (p_label, p_style) = if item.provider.contains('+') {
                ("[MULTI] ", Style::default().fg(Color::Yellow).bold())
            } else if item.provider.contains("Torrentio") {
                ("[TORR]  ", Style::default().fg(Color::Cyan).bold())
            } else if item.provider.contains("PirateBay") {
                ("[TPB]   ", Style::default().fg(Color::Magenta).bold())
            } else if item.provider.contains("Torrents-CSV") {
                ("[CSV]   ", Style::default().fg(Color::Green).bold())
            } else {
                ("[SRC]   ", Style::default().fg(Color::Gray))
            };
            let provider_span = Span::styled(p_label, p_style);

            let res_style = match item.resolution.as_str() {
                "4K" => Style::default().fg(Color::Magenta).bold(),
                "1080p" => Style::default().fg(Color::Green).bold(),
                "720p" => Style::default().fg(Color::Cyan),
                _ => Style::default().fg(Color::DarkGray),
            };
            let res_span = Span::styled(format!("[{:5}] ", item.resolution), res_style);

            let seeds_span = match item.seeds {
                Some(s) if s >= 100 => {
                    Span::styled(format!("▲ {:4} ", s), Style::default().fg(Color::Green))
                }
                Some(s) if s >= 20 => {
                    Span::styled(format!("▲ {:4} ", s), Style::default().fg(Color::Yellow))
                }
                Some(s) => Span::styled(format!("▲ {:4} ", s), Style::default().fg(Color::Red)),
                None => Span::styled("       ", Style::default().fg(Color::DarkGray)),
            };

            let size_span = Span::styled(format!("{:8} ", item.size_str), Style::default().fg(Color::Cyan));

            let dub_span = if item.is_foreign_dub_only() {
                Span::styled("[Dub] ", Style::default().fg(Color::Rgb(255, 100, 100)))
            } else {
                Span::raw("")
            };

            let title_span = Span::styled(&item.title, Style::default().fg(Color::White));

            ListItem::new(Line::from(vec![
                num_span, recom_span, h_span, provider_span, res_span, seeds_span, size_span, dub_span, title_span,
            ]))
        })
        .collect();

    let title_prefix = app.selected_show().and_then(|s| s.l.as_deref()).unwrap_or("Sources");
    let ep_prefix = if app.selected_show().is_some_and(|s| s.is_series()) {
        format!(" S{:02}E{:02}", app.season, app.episode)
    } else {
        String::new()
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(" {title_prefix}{ep_prefix} - Stream Sources ({}) ", filtered_indices.len()),
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
                        format!("{s} seeders (Excellent swarm health)")
                    } else if s >= 20 {
                        format!("{s} seeders (Good speed)")
                    } else {
                        format!("{s} seeders (May buffer / low peers)")
                    }
                })
                .unwrap_or_else(|| "Unknown peer count".to_string());

            let quality_text = if let Some(hdr) = &item.hdr_info {
                format!("{} ({hdr})", item.resolution)
            } else {
                item.resolution.clone()
            };

            let codec_text = item.codec.as_deref().unwrap_or("Standard / x264");

            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Stream Health: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} (Score: {} pts)", item.health().badge().0, item.smart_score()), Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Swarm Buffer:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(item.health().badge().1, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Audio / Dub:   ", Style::default().fg(Color::DarkGray)),
                    if item.is_foreign_dub_only() {
                        Span::styled("⚠️ Foreign Dub Only (Non-English)", Style::default().fg(Color::Red).bold())
                    } else {
                        Span::styled("✓ English / Multi-Audio Verified", Style::default().fg(Color::Green).bold())
                    },
                ]),
                Line::from(vec![
                    Span::styled("Resolution:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(quality_text, Style::default().fg(Color::Green).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Video Codec:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(codec_text, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("P2P Health:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(seeds_text, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("File Size:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.size_str, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Provider:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.provider, Style::default().fg(Color::Magenta).bold()),
                ]),
                Line::from(""),
                Line::from(Span::styled("Release Title:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&item.title, Style::default().fg(Color::White).bold())),
                Line::from(""),
                Line::from(Span::styled("Subtitles: Auto-attached via OpenSubtitles v3", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("Player:    MPV / HW Acceleration (libtorrent HTTP daemon)", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled(
                    "▶ Press [Enter] to stream directly (with auto-failover)",
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
// Anime Screens
// ----------------------------------------------------------------------------

fn render_anime_search_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // Results list & details
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            " ⛩  Search Anime (Kitsu • Nyaa • Torrentio) ",
            Style::default().fg(Color::Magenta).bold(),
        ));

    let input_line = if app.anime_query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Type anime name (e.g. Frieren, Attack on Titan, Demon Slayer, Jujutsu Kaisen, Ghibli)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.anime_query, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Magenta)),
        ])
    };

    let search_p = Paragraph::new(input_line).block(search_block);
    f.render_widget(search_p, chunks[1]);

    f.set_cursor_position((
        chunks[1].x + 4 + (app.anime_query.len() as u16).min(chunks[1].width.saturating_sub(6)),
        chunks[1].y + 1,
    ));

    if app.anime_results.is_empty() {
        let empty_card = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                if app.anime_query.is_empty() {
                    "Loading trending anime catalog from Kitsu..."
                } else {
                    "No anime found matching your query."
                },
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "• Press [Enter] to submit search or check spelling",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "• Press [Esc] to clear search",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Anime Catalog "),
        );
        f.render_widget(empty_card, chunks[2]);
        return;
    }

    let is_wide = chunks[2].width >= 80;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[2]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[2], None)
    };

    let items: Vec<ListItem> = app
        .anime_results
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let year = &a.year;
            let title = a.display_title();
            let is_movie = a.is_movie();

            let kind_badge = if is_movie {
                Span::styled(" [MOVIE]  ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::styled(format!(" [{:^7}] ", a.subtype_label()), Style::default().fg(Color::Magenta).bold())
            };

            let score_span = if let Some(r) = &a.rating {
                Span::styled(format!(" ⭐{r:>4}%"), Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            };

            let eps_badge = Span::styled(format!(" [{}]", a.episodes_display()), Style::default().fg(Color::Cyan));

            let num_span = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let title_span = Span::styled(title, Style::default().fg(Color::White).bold());
            let year_span = Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray));

            ListItem::new(Line::from(vec![
                num_span,
                kind_badge,
                title_span,
                year_span,
                eps_badge,
                score_span,
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            format!(" Anime Titles ({} found) ", app.anime_results.len()),
            Style::default().fg(Color::Magenta).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(55, 25, 60))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.anime_state.clone());

    if let Some(detail_box) = details_area {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Anime Information ", Style::default().fg(Color::White).bold()));

        if let Some(anime) = app.selected_anime() {
            let title = anime.display_title();
            let romaji = &anime.title;
            let jp = anime.title_jp.as_deref().unwrap_or("");
            let year = &anime.year;
            let kind = anime.subtype_label();
            let status = anime.status_label();
            let eps = anime.episodes_display();
            let rating_str = anime
                .rating
                .as_deref()
                .map(|r| format!("⭐ {r}% community rating"))
                .unwrap_or_else(|| "Rating N/A".to_string());
            let age_guide = anime.age_rating_guide.as_deref().unwrap_or("All Ages");

            let action_hint = if anime.is_movie() {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to stream Movie directly in MPV", Style::default().fg(Color::Yellow).bold()),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to choose Episode", Style::default().fg(Color::Magenta).bold()),
                ])
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(title, Style::default().fg(Color::White).bold().add_modifier(Modifier::UNDERLINED)),
                    Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray)),
                ]),
            ];

            if !jp.is_empty() && jp != title {
                lines.push(Line::from(Span::styled(format!("Kanji/Alt: {jp}"), Style::default().fg(Color::DarkGray).italic())));
            } else if romaji != title {
                lines.push(Line::from(Span::styled(format!("Romaji: {romaji}"), Style::default().fg(Color::DarkGray).italic())));
            }

            lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Format:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(kind, Style::default().fg(Color::Magenta).bold()),
                    Span::styled("   Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(status, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Episodes:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(eps, Style::default().fg(Color::Cyan).bold()),
                    Span::styled("   Age Guide: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(age_guide, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("Score:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(rating_str, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Synopsis:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&anime.synopsis, Style::default().fg(Color::White))),
                Line::from(""),
                action_hint,
            ]);

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("Select an anime title to view details").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

fn render_anime_episode_screen(f: &mut Frame, app: &App, area: Rect) {
    let anime = app.selected_anime();
    let title = anime.map(|a| a.display_title()).unwrap_or("Anime");
    let year = anime.map(|a| a.year.as_str()).unwrap_or("----");
    let total_eps = anime.and_then(|a| a.episode_count);

    let card_area = centered_rect(65, 55, area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(" Episode Selection ", Style::default().fg(Color::Magenta).bold()));

    let inner = main_block.inner(card_area);
    f.render_widget(main_block, card_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title info
            Constraint::Length(3), // Episode stepper
            Constraint::Length(2), // Target badge
            Constraint::Min(0),    // Control hints
        ])
        .split(inner);

    let title_line = Paragraph::new(Line::from(vec![
        Span::styled("Anime: ", Style::default().fg(Color::DarkGray)),
        Span::styled(title, Style::default().fg(Color::White).bold()),
        Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title_line, rows[0]);

    // Episode Stepper Box
    let ep_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta).bold())
        .title(Span::styled("▶ Episode Stepper", Style::default().fg(Color::Magenta).bold()));

    let max_str = if let Some(m) = total_eps {
        format!(" / {m}")
    } else {
        String::new()
    };

    let ep_content = Paragraph::new(Line::from(vec![
        Span::styled("◄  ", Style::default().fg(Color::Magenta).bold()),
        Span::styled(format!("Episode {:02}{max_str}", app.anime_episode), Style::default().fg(Color::White).bold()),
        Span::styled("  ►", Style::default().fg(Color::Magenta).bold()),
        Span::styled("   (← / → adjust, or type episode number)", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(ep_box);
    f.render_widget(ep_content, rows[1]);

    // Target badge
    let target_str = if let Some(m) = total_eps {
        format!("Episode {} of {}", app.anime_episode, m)
    } else {
        format!("Episode {}", app.anime_episode)
    };

    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Streaming Target: ", Style::default().fg(Color::DarkGray)),
        Span::styled(target_str, Style::default().fg(Color::Yellow).bold()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(preview, rows[2]);

    let hints = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("[← / →, - / +] ", Style::default().fg(Color::Magenta)),
            Span::raw("Adjust Episode    "),
            Span::styled("[0-9] ", Style::default().fg(Color::Magenta)),
            Span::raw("Type Number"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(Color::Green).bold()),
            Span::styled("⚡ Instant Play (Auto-Best)    ", Style::default().fg(Color::Green).bold()),
            Span::styled("[v] ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Browse Sources    ", Style::default().fg(Color::Yellow)),
            Span::styled("[q / Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Back to Anime List"),
        ]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hints, rows[3]);
}

fn render_anime_sources_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter & Sort Bar
            Constraint::Min(0),    // Main sources list & inspector
        ])
        .split(area);

    let q_filters = [
        QualityFilter::All,
        QualityFilter::Res4k,
        QualityFilter::Res1080p,
        QualityFilter::Res720p,
        QualityFilter::Res480p,
    ];

    let mut filter_spans = vec![
        Span::styled(" Quality [Tab/f]: ", Style::default().fg(Color::DarkGray)),
    ];

    for qf in q_filters {
        let count = app.anime_sources.iter().filter(|s| qf.matches(s)).count();
        let is_active = app.anime_quality_filter == qf;
        let (bullet, style) = if is_active {
            ("● ", Style::default().fg(Color::Black).bg(Color::Magenta).bold())
        } else {
            ("○ ", Style::default().fg(Color::Gray))
        };
        let label = qf.label();
        filter_spans.push(Span::styled(format!(" [{bullet}{label}:{count}] "), style));
        filter_spans.push(Span::raw(" "));
    }

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Provider [p]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.anime_provider_filter.label()),
        Style::default().fg(Color::Yellow).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Sort [s]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.anime_sort_order.label()),
        Style::default().fg(Color::Green).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Dubs [e]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        if app.hide_foreign_dubs { " [Hide] " } else { " [Show] " },
        if app.hide_foreign_dubs {
            Style::default().fg(Color::Green).bold()
        } else {
            Style::default().fg(Color::Yellow).bold()
        },
    ));

    let filter_bar = Paragraph::new(Line::from(filter_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(filter_bar, chunks[0]);

    let filtered_indices = app.filtered_anime_source_indices();

    if filtered_indices.is_empty() {
        let (heading, hint) = if app.anime_sources.is_empty() {
            (
                "No anime sources found across active indexers.",
                "Torrentio (Nyaa, AniDex, HorribleSubs) and NyaaSi direct RSS returned 0 streams. Press [Esc] to choose another episode or title.",
            )
        } else {
            (
                "No sources match the selected quality or provider filter.",
                "Press [f] to cycle quality or [p] to reset provider filter to All.",
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
                .title(" Anime Sources "),
        );
        f.render_widget(empty_msg, chunks[1]);
        return;
    }

    let is_wide = chunks[1].width >= 85;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[1]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[1], None)
    };

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &actual_idx)| {
            let s = &app.anime_sources[actual_idx];
            let num_span = Span::styled(format!("{:2}. ", display_idx + 1), Style::default().fg(Color::DarkGray));

            let recom_span = if display_idx == 0 {
                Span::styled("★ TOP ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::raw("      ")
            };

            let (h_tag, _) = s.health().badge();
            let h_span = match s.health() {
                crate::models::StreamHealth::Instant => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green).bold()),
                crate::models::StreamHealth::Good => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green)),
                crate::models::StreamHealth::Fair => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Yellow)),
                crate::models::StreamHealth::Slow => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Rgb(255, 165, 0))),
                crate::models::StreamHealth::Stalled => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Red)),
            };

            let res_style = match s.resolution.as_str() {
                "4K" => Style::default().fg(Color::Black).bg(Color::Magenta).bold(),
                "1080p" => Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
                "720p" => Style::default().fg(Color::Black).bg(Color::Blue).bold(),
                _ => Style::default().fg(Color::Black).bg(Color::DarkGray),
            };
            let res_span = Span::styled(format!(" {:^5} ", s.resolution), res_style);

            let seeds_val = s.seeds.unwrap_or(0);
            let seeds_style = if seeds_val > 50 {
                Style::default().fg(Color::Green).bold()
            } else if seeds_val > 10 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let seeds_span = Span::styled(format!(" 👤{:>3} ", seeds_val), seeds_style);

            let size_span = Span::styled(format!(" 💾{:>8} ", s.size_str), Style::default().fg(Color::Gray));

            let dub_span = if s.is_foreign_dub_only() {
                Span::styled(" [Dub] ", Style::default().fg(Color::Rgb(255, 100, 100)))
            } else {
                Span::raw("")
            };

            let title_span = Span::styled(&s.title, Style::default().fg(Color::White));

            ListItem::new(Line::from(vec![
                num_span,
                recom_span,
                h_span,
                res_span,
                seeds_span,
                size_span,
                dub_span,
                title_span,
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            format!(" Streams ({} / {} available) ", filtered_indices.len(), app.anime_sources.len()),
            Style::default().fg(Color::Magenta).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(55, 25, 60))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.anime_src_state.clone());

    if let Some(detail_box) = details_area {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Stream Inspector ", Style::default().fg(Color::White).bold()));

        let selected_item = filtered_indices
            .get(app.anime_src_idx)
            .and_then(|&idx| app.anime_sources.get(idx));

        if let Some(src) = selected_item {
            let anime = app.selected_anime();
            let anime_title = anime.map(|a| a.display_title()).unwrap_or("Anime");
            let is_movie = anime.is_some_and(|a| a.is_movie());

            let target_title = if is_movie {
                format!("{anime_title} (Movie)")
            } else {
                format!("{anime_title} - Episode {:02}", app.anime_episode)
            };

            let seeds_val = src.seeds.unwrap_or(0);
            let health_badge = if seeds_val >= 25 {
                Span::styled(" ● High Health (Fast P2P Swarm) ", Style::default().fg(Color::Green).bold())
            } else if seeds_val >= 5 {
                Span::styled(" ● Moderate Health ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::styled(" ○ Low Health (Buffering may take 30-60s) ", Style::default().fg(Color::Red))
            };

            let info_hash = match &src.stream {
                StreamSource::Torrent { info_hash, .. } => info_hash.clone(),
                StreamSource::DirectUrl { url } => url.clone(),
                StreamSource::SssPick { num } => format!("Direct Index #{num}"),
            };

            let file_idx_str = match &src.stream {
                StreamSource::Torrent { file_idx: Some(idx), .. } => format!("File #{idx}"),
                StreamSource::Torrent { file_idx: None, .. } => "Default (file #0)".to_string(),
                _ => "N/A".to_string(),
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Stream Target: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(target_title, Style::default().fg(Color::White).bold()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Swarm Health:  ", Style::default().fg(Color::DarkGray)),
                    health_badge,
                ]),
                Line::from(vec![
                    Span::styled("Seeders:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{seeds_val} active seeders"), Style::default().fg(Color::White).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Quality:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.resolution, Style::default().fg(Color::Magenta).bold()),
                    Span::raw("  "),
                    Span::styled(src.codec.as_deref().unwrap_or("x264/AVC"), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Exact Size:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.size_str, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Provider:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.provider, Style::default().fg(Color::Yellow).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Target File:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(file_idx_str, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Release Title / Filename:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&src.title, Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Info Hash:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(info_hash, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
            ];

            if !is_movie {
                lines.push(Line::from(vec![
                    Span::styled("Binge-Watch:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[N] Next Episode  ", Style::default().fg(Color::Green).bold()),
                    Span::styled("[P] Previous Episode", Style::default().fg(Color::Yellow)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled("Controls:      ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter] Launch MPV Stream  ", Style::default().fg(Color::Yellow).bold()),
                Span::styled("[q / Esc] Back", Style::default().fg(Color::DarkGray)),
            ]));

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("Select a source to inspect swarm details").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

// ----------------------------------------------------------------------------
// Cartoons & Animated Shows Screens
// ----------------------------------------------------------------------------

fn render_cartoon_search_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // Results list & details
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            " 🎨 Search Cartoons & Animated Shows (TVMaze • IMDb • Torrentio) ",
            Style::default().fg(Color::Yellow).bold(),
        ));

    let input_line = if app.cartoon_query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Type cartoon name (e.g. Adventure Time, Rick and Morty, Avatar, Gravity Falls, Spider-Verse)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.cartoon_query, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Yellow)),
        ])
    };

    let search_p = Paragraph::new(input_line).block(search_block);
    f.render_widget(search_p, chunks[1]);

    f.set_cursor_position((
        chunks[1].x + 4 + (app.cartoon_query.len() as u16).min(chunks[1].width.saturating_sub(6)),
        chunks[1].y + 1,
    ));

    if app.cartoon_results.is_empty() {
        let empty_card = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                if app.cartoon_query.is_empty() {
                    "Loading popular cartoons & animated shows catalog..."
                } else {
                    "No animated shows found matching your query."
                },
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "• Press [Enter] to submit search or check spelling",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "• Press [Esc] to clear search",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Cartoons Catalog "),
        );
        f.render_widget(empty_card, chunks[2]);
        return;
    }

    let is_wide = chunks[2].width >= 80;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[2]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[2], None)
    };

    let items: Vec<ListItem> = app
        .cartoon_results
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let year = &c.year;
            let title = &c.title;
            let network = c.display_network();

            let network_style = match network {
                "Cartoon Network" => Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
                "Adult Swim" => Style::default().fg(Color::Black).bg(Color::Green).bold(),
                "Nickelodeon" => Style::default().fg(Color::Black).bg(Color::Rgb(255, 140, 0)).bold(),
                "Disney Channel" => Style::default().fg(Color::Black).bg(Color::Blue).bold(),
                "Netflix" => Style::default().fg(Color::Black).bg(Color::Red).bold(),
                _ => Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
            };
            let net_badge = Span::styled(format!(" [{network}] "), network_style);

            let score_span = if let Some(r) = &c.rating {
                Span::styled(format!(" ⭐{r:>4}"), Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            };

            let num_span = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let title_span = Span::styled(title, Style::default().fg(Color::White).bold());
            let year_span = Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray));

            ListItem::new(Line::from(vec![
                num_span,
                net_badge,
                title_span,
                year_span,
                score_span,
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            format!(" Animated Titles ({} found) ", app.cartoon_results.len()),
            Style::default().fg(Color::Yellow).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 50, 15))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.cartoon_state.clone());

    if let Some(detail_box) = details_area {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Cartoon Details ", Style::default().fg(Color::White).bold()));

        if let Some(cartoon) = app.selected_cartoon() {
            let title = &cartoon.title;
            let year = &cartoon.year;
            let network = cartoon.display_network();
            let status = &cartoon.status;
            let genres = cartoon.genres_str();
            let imdb_id = cartoon.imdb_id.as_deref().unwrap_or("N/A");
            let rating_str = cartoon
                .rating
                .as_deref()
                .map(|r| format!("⭐ {r} / 10"))
                .unwrap_or_else(|| "Rating N/A".to_string());

            let action_hint = if cartoon.is_series {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to choose Season & Episode", Style::default().fg(Color::Yellow).bold()),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Press [Enter] to stream Movie directly in MPV", Style::default().fg(Color::Green).bold()),
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
                    Span::styled("Network:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(network, Style::default().fg(Color::Yellow).bold()),
                    Span::styled("   Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(status, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Genres:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(genres, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("IMDb ID:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(imdb_id, Style::default().fg(Color::Gray)),
                    Span::styled("   Score: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(rating_str, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Synopsis:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&cartoon.summary, Style::default().fg(Color::White))),
                Line::from(""),
                action_hint,
            ];

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("Select a cartoon to view details").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

fn render_cartoon_season_episode_screen(f: &mut Frame, app: &App, area: Rect) {
    let cartoon = app.selected_cartoon();
    let title = cartoon.map(|c| c.title.as_str()).unwrap_or("Cartoon");
    let year = cartoon.map(|c| c.year.as_str()).unwrap_or("----");
    let network = cartoon.map(|c| c.display_network()).unwrap_or("Animation");

    let card_area = centered_rect(65, 55, area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(" Season & Episode Selection ", Style::default().fg(Color::Yellow).bold()));

    let inner = main_block.inner(card_area);
    f.render_widget(main_block, card_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title info
            Constraint::Length(3), // Season stepper
            Constraint::Length(1), // Spacing
            Constraint::Length(3), // Episode stepper
            Constraint::Length(2), // Target badge
            Constraint::Min(0),    // Control hints
        ])
        .split(inner);

    let title_line = Paragraph::new(Line::from(vec![
        Span::styled(format!("{network} • "), Style::default().fg(Color::Yellow)),
        Span::styled(title, Style::default().fg(Color::White).bold()),
        Span::styled(format!(" ({year})"), Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title_line, rows[0]);

    // Season Stepper Box
    let is_season_active = app.cartoon_stepper_field == StepperField::Season;
    let s_border_style = if is_season_active {
        Style::default().fg(Color::Yellow).bold()
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
        Span::styled("◄  ", if is_season_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled(format!("{:02}", app.cartoon_season), Style::default().fg(Color::White).bold()),
        Span::styled("  ►", if is_season_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled("   (← / → adjust, or type number)", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(s_box);
    f.render_widget(s_content, rows[1]);

    // Episode Stepper Box
    let is_ep_active = app.cartoon_stepper_field == StepperField::Episode;
    let ep_border_style = if is_ep_active {
        Style::default().fg(Color::Yellow).bold()
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
        Span::styled("◄  ", if is_ep_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled(format!("{:02}", app.cartoon_episode), Style::default().fg(Color::White).bold()),
        Span::styled("  ►", if is_ep_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }),
        Span::styled("   (← / → adjust, or type number)", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center)
    .block(ep_box);
    f.render_widget(ep_content, rows[3]);

    // Target badge
    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Streaming Target: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Season {}, Episode {}  (S{:02}E{:02})", app.cartoon_season, app.cartoon_episode, app.cartoon_season, app.cartoon_episode),
            Style::default().fg(Color::Yellow).bold(),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(preview, rows[4]);

    let hints = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("[Tab / ↑ / ↓] ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch Field    "),
            Span::styled("[← / →] ", Style::default().fg(Color::Yellow)),
            Span::raw("Adjust    "),
            Span::styled("[0-9] ", Style::default().fg(Color::Yellow)),
            Span::raw("Type Number"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(Color::Green).bold()),
            Span::styled("⚡ Instant Play (Auto-Best)    ", Style::default().fg(Color::Green).bold()),
            Span::styled("[v] ", Style::default().fg(Color::Yellow).bold()),
            Span::styled("Browse Sources    ", Style::default().fg(Color::Yellow)),
            Span::styled("[q / Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Back to Cartoon List"),
        ]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hints, rows[5]);
}

fn render_cartoon_sources_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter & Sort Bar
            Constraint::Min(0),    // Main sources list & inspector
        ])
        .split(area);

    let q_filters = [
        QualityFilter::All,
        QualityFilter::Res4k,
        QualityFilter::Res1080p,
        QualityFilter::Res720p,
        QualityFilter::Res480p,
    ];

    let mut filter_spans = vec![
        Span::styled(" Quality [Tab/f]: ", Style::default().fg(Color::DarkGray)),
    ];

    for qf in q_filters {
        let count = app.cartoon_sources.iter().filter(|s| qf.matches(s)).count();
        let is_active = app.cartoon_quality_filter == qf;
        let (bullet, style) = if is_active {
            ("● ", Style::default().fg(Color::Black).bg(Color::Yellow).bold())
        } else {
            ("○ ", Style::default().fg(Color::Gray))
        };
        let label = qf.label();
        filter_spans.push(Span::styled(format!(" [{bullet}{label}:{count}] "), style));
        filter_spans.push(Span::raw(" "));
    }

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Provider [p]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.cartoon_provider_filter.label()),
        Style::default().fg(Color::Yellow).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Sort [s]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        format!(" [{}] ", app.cartoon_sort_order.label()),
        Style::default().fg(Color::Green).bold(),
    ));

    filter_spans.push(Span::raw(" │ "));
    filter_spans.push(Span::styled(" Dubs [e]: ", Style::default().fg(Color::DarkGray)));
    filter_spans.push(Span::styled(
        if app.hide_foreign_dubs { " [Hide] " } else { " [Show] " },
        if app.hide_foreign_dubs {
            Style::default().fg(Color::Green).bold()
        } else {
            Style::default().fg(Color::Yellow).bold()
        },
    ));

    let filter_bar = Paragraph::new(Line::from(filter_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(filter_bar, chunks[0]);

    let filtered_indices = app.filtered_cartoon_source_indices();

    if filtered_indices.is_empty() {
        let (heading, hint) = if app.cartoon_sources.is_empty() {
            (
                "No cartoon sources found across active indexers.",
                "Torrentio, The Pirate Bay, and Torrents-CSV returned 0 streams. Press [Esc] to choose another season, episode, or title.",
            )
        } else {
            (
                "No sources match the selected quality or provider filter.",
                "Press [f] to cycle quality or [p] to reset provider filter to All.",
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
                .title(" Cartoon Sources "),
        );
        f.render_widget(empty_msg, chunks[1]);
        return;
    }

    let is_wide = chunks[1].width >= 85;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[1]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[1], None)
    };

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &actual_idx)| {
            let s = &app.cartoon_sources[actual_idx];
            let num_span = Span::styled(format!("{:2}. ", display_idx + 1), Style::default().fg(Color::DarkGray));

            let recom_span = if display_idx == 0 {
                Span::styled("★ TOP ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::raw("      ")
            };

            let (h_tag, _) = s.health().badge();
            let h_span = match s.health() {
                crate::models::StreamHealth::Instant => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green).bold()),
                crate::models::StreamHealth::Good => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Green)),
                crate::models::StreamHealth::Fair => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Yellow)),
                crate::models::StreamHealth::Slow => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Rgb(255, 165, 0))),
                crate::models::StreamHealth::Stalled => Span::styled(format!("{h_tag:10} "), Style::default().fg(Color::Red)),
            };

            let res_style = match s.resolution.as_str() {
                "4K" => Style::default().fg(Color::Black).bg(Color::Magenta).bold(),
                "1080p" => Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
                "720p" => Style::default().fg(Color::Black).bg(Color::Blue).bold(),
                _ => Style::default().fg(Color::Black).bg(Color::DarkGray),
            };
            let res_span = Span::styled(format!(" {:^5} ", s.resolution), res_style);

            let seeds_val = s.seeds.unwrap_or(0);
            let seeds_style = if seeds_val > 50 {
                Style::default().fg(Color::Green).bold()
            } else if seeds_val > 10 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let seeds_span = Span::styled(format!(" 👤{:>3} ", seeds_val), seeds_style);

            let size_span = Span::styled(format!(" 💾{:>8} ", s.size_str), Style::default().fg(Color::Gray));

            let dub_span = if s.is_foreign_dub_only() {
                Span::styled(" [Dub] ", Style::default().fg(Color::Rgb(255, 100, 100)))
            } else {
                Span::raw("")
            };

            let title_span = Span::styled(&s.title, Style::default().fg(Color::White));

            ListItem::new(Line::from(vec![
                num_span,
                recom_span,
                h_span,
                res_span,
                seeds_span,
                size_span,
                dub_span,
                title_span,
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            format!(" Streams ({} / {} available) ", filtered_indices.len(), app.cartoon_sources.len()),
            Style::default().fg(Color::Yellow).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 50, 15))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.cartoon_src_state.clone());

    if let Some(detail_box) = details_area {
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Stream Inspector ", Style::default().fg(Color::White).bold()));

        let selected_item = filtered_indices
            .get(app.cartoon_src_idx)
            .and_then(|&idx| app.cartoon_sources.get(idx));

        if let Some(src) = selected_item {
            let cartoon = app.selected_cartoon();
            let cartoon_title = cartoon.map(|c| c.title.as_str()).unwrap_or("Cartoon");
            let is_series = cartoon.is_some_and(|c| c.is_series);

            let target_title = if is_series {
                format!("{cartoon_title} - S{:02}E{:02}", app.cartoon_season, app.cartoon_episode)
            } else {
                format!("{cartoon_title} (Animated Movie)")
            };

            let seeds_val = src.seeds.unwrap_or(0);
            let health_badge = if seeds_val >= 25 {
                Span::styled(" ● High Health (Fast P2P Swarm) ", Style::default().fg(Color::Green).bold())
            } else if seeds_val >= 5 {
                Span::styled(" ● Moderate Health ", Style::default().fg(Color::Yellow).bold())
            } else {
                Span::styled(" ○ Low Health (Buffering may take 30-60s) ", Style::default().fg(Color::Red))
            };

            let info_hash = match &src.stream {
                StreamSource::Torrent { info_hash, .. } => info_hash.clone(),
                StreamSource::DirectUrl { url } => url.clone(),
                StreamSource::SssPick { num } => format!("Direct Index #{num}"),
            };

            let file_idx_str = match &src.stream {
                StreamSource::Torrent { file_idx: Some(idx), .. } => format!("File #{idx}"),
                StreamSource::Torrent { file_idx: None, .. } => "Default (file #0)".to_string(),
                _ => "N/A".to_string(),
            };

            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Stream Target: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(target_title, Style::default().fg(Color::White).bold()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Swarm Health:  ", Style::default().fg(Color::DarkGray)),
                    health_badge,
                ]),
                Line::from(vec![
                    Span::styled("Seeders:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{seeds_val} active seeders"), Style::default().fg(Color::White).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Quality:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.resolution, Style::default().fg(Color::Yellow).bold()),
                    Span::raw("  "),
                    Span::styled(src.codec.as_deref().unwrap_or("x264/AVC"), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Exact Size:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.size_str, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Provider:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&src.provider, Style::default().fg(Color::Green).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Target File:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(file_idx_str, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Release Title / Filename:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled(&src.title, Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Info Hash:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(info_hash, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
            ];

            if is_series {
                lines.push(Line::from(vec![
                    Span::styled("Binge-Watch:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[N] Next Episode  ", Style::default().fg(Color::Green).bold()),
                    Span::styled("[P] Previous Episode", Style::default().fg(Color::Yellow)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled("Controls:      ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Enter] Launch MPV Stream  ", Style::default().fg(Color::Yellow).bold()),
                Span::styled("[q / Esc] Back", Style::default().fg(Color::DarkGray)),
            ]));

            let p = Paragraph::new(lines)
                .block(detail_block)
                .wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("Select a source to inspect swarm details").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

// ----------------------------------------------------------------------------
// Live TV (IPTV) Screen
// ----------------------------------------------------------------------------

fn render_iptv_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Category Bar & Filter
            Constraint::Length(3), // Search Input
            Constraint::Min(0),    // Channel List & Inspector
        ])
        .split(area);

    // --- Category Tabs ---
    let categories = [
        "All", "News", "Movies", "Sports", "Entertainment", "Music", "Documentary", "Kids", "General"
    ];

    let mut cat_spans = vec![
        Span::styled(" Category [Tab / [ / ]]: ", Style::default().fg(Color::DarkGray)),
    ];

    for cat in categories {
        let is_active = app.iptv_category == cat;
        let (bullet, style) = if is_active {
            ("● ", Style::default().fg(Color::Black).bg(Color::Green).bold())
        } else {
            ("○ ", Style::default().fg(Color::Gray))
        };
        cat_spans.push(Span::styled(format!(" [{bullet}{cat}] "), style));
        cat_spans.push(Span::raw(" "));
    }

    let cat_bar = Paragraph::new(Line::from(cat_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(cat_bar, chunks[0]);

    // --- Channel Search Box ---
    let input_line = if app.iptv_query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Filter channels by name or country (e.g. BBC, Sky, ESPN, CNN, Pluto, France 24)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.iptv_query, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Green)),
        ])
    };

    let search_box = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green))
            .title(Span::styled(" 📺 Filter Live Channels ", Style::default().fg(Color::Green).bold())),
    );
    f.render_widget(search_box, chunks[1]);

    // --- Channel List & Inspector ---
    let filtered = app.filtered_iptv_channels();

    if filtered.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No channels matched your search filter.", Style::default().fg(Color::Yellow).bold())),
            Line::from(""),
            Line::from(Span::styled("Try clearing the search query or selecting 'All' categories.", Style::default().fg(Color::DarkGray))),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Channels "),
        );
        f.render_widget(empty_msg, chunks[2]);
        return;
    }

    let is_wide = chunks[2].width >= 85;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[2]);
        (cols[0], Some(cols[1]))
    } else {
        (chunks[2], None)
    };

    let items: Vec<ListItem> = filtered
        .iter()
        .take(500)
        .enumerate()
        .map(|(i, ch)| {
            let num = Span::styled(format!("{:3}. ", i + 1), Style::default().fg(Color::DarkGray));
            let grp = Span::styled(format!("[{:13}] ", truncate(&ch.group, 13)), Style::default().fg(Color::Cyan));
            let res = Span::styled(format!("[{:5}] ", ch.resolution), Style::default().fg(Color::Green));
            let name = Span::styled(&ch.name, Style::default().fg(Color::White).bold());
            ListItem::new(Line::from(vec![num, grp, res, name]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            format!(" Live Channels ({} shown) ", filtered.len()),
            Style::default().fg(Color::Green).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(20, 50, 30))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut app.iptv_state.clone());

    if let Some(detail_box) = details_area {
        let selected_ch = filtered.get(app.iptv_idx);
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(" Channel Inspector ", Style::default().fg(Color::White).bold()));

        if let Some(ch) = selected_ch {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(&ch.name, Style::default().fg(Color::White).bold())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Category:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&ch.group, Style::default().fg(Color::Cyan).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Quality:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&ch.resolution, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Stream URL: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(truncate(&ch.url, 45), Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Source: IPTV-org Global Directory (11k+ channels)", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("Player: MPV HLS streaming with auto-reconnect", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled("▶ Press [Enter] to watch live stream", Style::default().fg(Color::Green).bold())),
            ];

            let p = Paragraph::new(lines).block(detail_block).wrap(Wrap { trim: true });
            f.render_widget(p, detail_box);
        } else {
            let p = Paragraph::new("No channel selected").block(detail_block);
            f.render_widget(p, detail_box);
        }
    }
}

// ----------------------------------------------------------------------------
// Internet Archive Screen
// ----------------------------------------------------------------------------

fn render_archive_search_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Search Input Box
            Constraint::Length(1), // Spacing
            Constraint::Min(0),    // Results list or info card
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(" 🏛 Search Internet Archive (Public Domain & Vintage Films) ", Style::default().fg(Color::Magenta).bold()));

    let input_line = if app.archive_query.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Type movie or documentary keyword (e.g. Night of the Living Dead, Chaplin, Metropolis, Plan 9)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.archive_query, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Magenta)),
        ])
    };

    let search_p = Paragraph::new(input_line).block(search_block);
    f.render_widget(search_p, chunks[1]);

    if app.archive_results.is_empty() {
        let info_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(" Internet Archive Overview ", Style::default().fg(Color::Gray)));

        let tips = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  • Direct Public-Domain Streaming: ", Style::default().fg(Color::Magenta).bold()),
                Span::raw("Stream classic movies, silent cinema, and historical docs directly via MP4."),
            ]),
            Line::from(vec![
                Span::styled("  • No Torrents / No Waiting: ", Style::default().fg(Color::Green).bold()),
                Span::raw("High speed direct HTTP seekable playback, independent of P2P seeders."),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Popular: Night of the Living Dead, Plan 9 from Outer Space, His Girl Friday, Carnival of Souls, Charlie Chaplin",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let p = Paragraph::new(tips).block(info_block);
        f.render_widget(p, chunks[3]);
        return;
    }

    let items: Vec<ListItem> = app
        .archive_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let num = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let year = Span::styled(format!("({:4}) ", r.year), Style::default().fg(Color::Gray));
            let dl = Span::styled(format!("⬇ {:6} dl ", r.downloads), Style::default().fg(Color::Yellow));
            let title = Span::styled(&r.title, Style::default().fg(Color::White).bold());
            ListItem::new(Line::from(vec![num, year, dl, title]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            format!(" Archive.org Matches ({}) - [Enter] to choose video file ", app.archive_results.len()),
            Style::default().fg(Color::Magenta).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 20, 50))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[3], &mut app.archive_state.clone());
}

fn render_archive_files_screen(f: &mut Frame, app: &App, area: Rect) {
    let card_area = centered_rect(70, 60, area);

    let selected_item = app.archive_results.get(app.archive_idx);
    let title = selected_item.map(|i| i.title.as_str()).unwrap_or("Archive Item");

    let items: Vec<ListItem> = app
        .archive_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let num = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let size = Span::styled(format!("[{:8}] ", file.size_str), Style::default().fg(Color::Cyan));
            let format = Span::styled(format!("[{:7}] ", file.format), Style::default().fg(Color::Green));
            let name = Span::styled(&file.name, Style::default().fg(Color::White).bold());
            ListItem::new(Line::from(vec![num, size, format, name]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            format!(" Select Stream Quality / File: {title} "),
            Style::default().fg(Color::Magenta).bold(),
        ));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 20, 50))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, card_area, &mut app.archive_file_state.clone());
}

// ----------------------------------------------------------------------------
// Direct Input Screen
// ----------------------------------------------------------------------------

fn render_direct_input_screen(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin
            Constraint::Length(3), // Input Box
            Constraint::Length(1), // Spacing
            Constraint::Min(0),    // Info & format guidance
        ])
        .split(area);

    let input_line = if app.direct_url_input.is_empty() {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Paste Magnet link (magnet:?xt=urn:btih:...) or direct URL (http://.../stream.m3u8 or .mp4)...",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&app.direct_url_input, Style::default().fg(Color::White).bold()),
            Span::styled("▋", Style::default().fg(Color::Cyan)),
        ])
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" 🔗 Direct Stream URL / Magnet Link ", Style::default().fg(Color::Cyan).bold()));

    let p = Paragraph::new(input_line).block(input_block);
    f.render_widget(p, chunks[1]);

    // Format detector
    let trimmed = app.direct_url_input.trim();
    let detected = if trimmed.starts_with("magnet:") {
        Span::styled("Detected: Magnet Link (will stream via libtorrent sequential HTTP daemon)", Style::default().fg(Color::Green).bold())
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if trimmed.contains(".m3u8") {
            Span::styled("Detected: HLS Live Stream (.m3u8)", Style::default().fg(Color::Green).bold())
        } else {
            Span::styled("Detected: HTTP Video Stream", Style::default().fg(Color::Green).bold())
        }
    } else if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        Span::styled("Detected: 40-character BTIH InfoHash", Style::default().fg(Color::Green).bold())
    } else if !trimmed.is_empty() {
        Span::styled("Unrecognized format (will attempt MPV playback directly)", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("Waiting for input...", Style::default().fg(Color::DarkGray))
    };

    let guide_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Supported Formats ");

    let guide = vec![
        Line::from(""),
        Line::from(vec![Span::raw("  • Status: "), detected]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  • Torrent Magnet: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("magnet:?xt=urn:btih:<info_hash>&dn=<title>"),
        ]),
        Line::from(vec![
            Span::styled("  • Raw InfoHash:   ", Style::default().fg(Color::Magenta).bold()),
            Span::raw("Any 40-character hex BitTorrent infohash (automatically attaches public trackers)"),
        ]),
        Line::from(vec![
            Span::styled("  • Live HLS / m3u8:", Style::default().fg(Color::Green).bold()),
            Span::raw("https://example.com/playlist.m3u8"),
        ]),
        Line::from(vec![
            Span::styled("  • Direct Video:   ", Style::default().fg(Color::Yellow).bold()),
            Span::raw("https://example.com/video.mp4 or .mkv"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press [Enter] to stream immediately in MPV", Style::default().fg(Color::White).bold())),
    ];

    let guide_p = Paragraph::new(guide).block(guide_block);
    f.render_widget(guide_p, chunks[3]);
}

// ----------------------------------------------------------------------------
// Continue Watching / Watch History Screen
// ----------------------------------------------------------------------------

fn render_history_screen(f: &mut Frame, app: &App, area: Rect) {
    if app.history.is_empty() {
        let empty_card = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "🎬 No Watch History Yet",
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Your recently streamed Movies, Anime, Cartoons, Series, and IPTV channels",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                "will automatically appear here with progress tracking and 1-click continuation!",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [F1-F6] to browse media catalogs, or [?] for full help.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Continue Watching & Watch History "),
        );
        f.render_widget(empty_card, area);
        return;
    }

    let is_wide = area.width >= 80;
    let (list_area, details_area) = if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        (cols[0], Some(cols[1]))
    } else {
        (area, None)
    };

    let items: Vec<ListItem> = app
        .history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let badge_color = match entry.badge_label() {
                "ANIME" => Color::Magenta,
                "CARTOON" => Color::Green,
                "SERIES" => Color::Cyan,
                "MOVIE" => Color::Yellow,
                "IPTV" => Color::LightYellow,
                "ARCHIVE" => Color::LightBlue,
                _ => Color::White,
            };

            let badge_span = Span::styled(
                format!(" [{:7}] ", entry.badge_label()),
                Style::default().fg(badge_color).bold(),
            );
            let num_span = Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray));
            let title_span = Span::styled(&entry.title, Style::default().fg(Color::White).bold());
            let prog_span = Span::styled(format!(" {} ", entry.progress_str()), Style::default().fg(Color::Yellow));

            let next_pill = if let Some(next_str) = entry.next_episode_str() {
                Span::styled(format!(" [Next: {next_str}] "), Style::default().fg(Color::LightGreen).bold())
            } else {
                Span::raw("")
            };

            let time_span = Span::styled(format!(" ({})", entry.time_ago_str()), Style::default().fg(Color::DarkGray));

            ListItem::new(Line::from(vec![
                num_span,
                badge_span,
                title_span,
                prog_span,
                next_pill,
                time_span,
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(" Continue Watching ({} items) ", app.history.len()),
            Style::default().fg(Color::Cyan).bold(),
        ));

    let mut state = app.history_state.clone();
    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 45, 65))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, list_area, &mut state);

    if let Some(d_area) = details_area {
        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(" Stream Resume Details ", Style::default().fg(Color::Yellow).bold()));

        if let Some(entry) = app.history.get(app.history_idx) {
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Title: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&entry.title, Style::default().fg(Color::White).bold()),
                ]),
                Line::from(vec![
                    Span::styled(" Mode:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&entry.mode, Style::default().fg(Color::Cyan).bold()),
                ]),
            ];

            if let Some(yr) = &entry.year {
                lines.push(Line::from(vec![
                    Span::styled(" Year:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(yr, Style::default().fg(Color::Gray)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled(" Watch Progress: ", Style::default().fg(Color::DarkGray)),
                Span::styled(entry.progress_str(), Style::default().fg(Color::Yellow).bold()),
            ]));

            if let Some(next_str) = entry.next_episode_str() {
                lines.push(Line::from(vec![
                    Span::styled(" Next Episode:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(next_str, Style::default().fg(Color::LightGreen).bold()),
                    Span::styled("  (Press [n] to continue)", Style::default().fg(Color::DarkGray).italic()),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled(" Last Provider: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&entry.provider, Style::default().fg(Color::Green)),
            ]));

            lines.push(Line::from(vec![
                Span::styled(" Quality:       ", Style::default().fg(Color::DarkGray)),
                Span::styled(&entry.resolution, Style::default().fg(Color::Cyan)),
            ]));

            lines.push(Line::from(vec![
                Span::styled(" Stream Type:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    match &entry.stream {
                        StreamSource::Torrent { .. } => "BitTorrent Magnet / Swarm",
                        StreamSource::DirectUrl { .. } => "Direct HTTP / HLS Stream",
                        StreamSource::SssPick { .. } => "Stremio / Torrentio Resolver",
                    },
                    Style::default().fg(Color::Gray),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::styled(" Last Watched:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(entry.time_ago_str(), Style::default().fg(Color::Yellow)),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(" ─ Quick Actions ─", Style::default().fg(Color::DarkGray))));
            lines.push(Line::from(vec![
                Span::styled("  [Enter] ", Style::default().fg(Color::Yellow).bold()),
                Span::raw("Replay / Resume current item"),
            ]));
            if entry.next_episode_str().is_some() {
                lines.push(Line::from(vec![
                    Span::styled("  [n / N] ", Style::default().fg(Color::LightGreen).bold()),
                    Span::raw("Advance & play Next Episode!"),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  [d]     ", Style::default().fg(Color::Red).bold()),
                Span::raw("Remove this item from history"),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  [c]     ", Style::default().fg(Color::Magenta).bold()),
                Span::raw("Clear entire watch history"),
            ]));

            let p = Paragraph::new(lines).block(details_block).wrap(Wrap { trim: false });
            f.render_widget(p, d_area);
        } else {
            let p = Paragraph::new(Line::from("No item selected")).block(details_block);
            f.render_widget(p, d_area);
        }
    }
}

// ----------------------------------------------------------------------------
// Help Modal Overlay
// ----------------------------------------------------------------------------

fn render_help_modal(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" 📖 tsss-tui Keyboard Shortcuts & Features ", Style::default().fg(Color::Cyan).bold()));

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("  --- Global Navigation & Mode Switching ---", Style::default().fg(Color::Yellow).bold())),
        Line::from("    [F1] / [Ctrl+1]  Switch to Movies & TV Series"),
        Line::from("    [F2] / [Ctrl+2]  Switch to Anime Streaming (Kitsu • Nyaa • Torrentio)"),
        Line::from("    [F3] / [Ctrl+3]  Switch to Cartoons & Animation (TVMaze • Cartoons Catalog • Multi-Indexer)"),
        Line::from("    [F4] / [Ctrl+4]  Switch to Live TV (IPTV Channels)"),
        Line::from("    [F5] / [Ctrl+5]  Switch to Internet Archive (Public Domain)"),
        Line::from("    [F6] / [Ctrl+6]  Switch to Direct Magnet / URL Streamer"),
        Line::from("    [F7] / [Ctrl+7]  Switch to Continue Watching (Watch History)"),
        Line::from("    [?]              Toggle this Help modal"),
        Line::from("    [Ctrl+C]         Force quit application"),
        Line::from(""),
        Line::from(Span::styled("  --- Continue Watching (Watch History) ---", Style::default().fg(Color::Yellow).bold())),
        Line::from("    [↑ / ↓, j / k]   Navigate through watched items"),
        Line::from("    [Enter]          Resume / Replay selected title"),
        Line::from("    [n / N]          Advance and immediately fetch Next Episode!"),
        Line::from("    [d / Delete]     Delete entry from watch history"),
        Line::from("    [c]              Clear entire watch history"),
        Line::from(""),
        Line::from(Span::styled("  --- Cartoons & Animation Mode ---", Style::default().fg(Color::Green).bold())),
        Line::from("    [Type]           Search cartoon series & animated movies (curated popular list if empty)"),
        Line::from("    [↑ / ↓, j / k]   Navigate cartoon catalog or stream sources"),
        Line::from("    [1 - 9]          Quick-select cartoon or stream source directly"),
        Line::from("    [Tab / ↑ / ↓]    Switch Season / Episode in Season & Episode Stepper"),
        Line::from("    [← / →, - / +]   Adjust Season or Episode number"),
        Line::from("    [0 - 9]          Type season or episode number directly"),
        Line::from("    [Tab / f]        Cycle Quality filter (All -> 4K -> 1080p -> 720p -> 480p)"),
        Line::from("    [p]              Cycle Provider filter (Torrentio, TPB, Torrents-CSV, NyaaSi)"),
        Line::from("    [s]              Cycle Sort Order (By Seeders, Quality, or File Size)"),
        Line::from("    [n / N]          Binge-watching: Jump directly to Next Episode!"),
        Line::from("    [P]              Jump directly to Previous Episode"),
        Line::from("    [Enter]          Confirm title / Load sources / Stream in MPV"),
        Line::from("    [q / Esc]        Back to previous screen / Clear search"),
        Line::from(""),
        Line::from(Span::styled("  --- Anime Streaming Mode ---", Style::default().fg(Color::Magenta).bold())),
        Line::from("    [Type]           Search anime titles (auto-loads trending if empty)"),
        Line::from("    [↑ / ↓, j / k]   Navigate anime titles or stream sources"),
        Line::from("    [1 - 9]          Quick-select anime or source directly"),
        Line::from("    [← / →, - / +]   Adjust Episode number in Episode Stepper"),
        Line::from("    [0 - 9]          Type episode number directly"),
        Line::from("    [Tab / f]        Cycle Quality filter (All -> 4K -> 1080p -> 720p -> 480p)"),
        Line::from("    [p]              Cycle Provider filter (All, NyaaSi, Torrentio, TPB)"),
        Line::from("    [s]              Cycle Sort Order (By Seeders, Quality, or File Size)"),
        Line::from("    [n / N]          Binge-watching: Jump directly to Next Episode!"),
        Line::from("    [P]              Jump directly to Previous Episode"),
        Line::from("    [Enter]          Confirm title / Episode / Stream selected source in MPV"),
        Line::from("    [q / Esc]        Back to previous screen / Clear search"),
        Line::from(""),
        Line::from(Span::styled("  --- Movies & Series (Multi-Source Streaming) ---", Style::default().fg(Color::Yellow).bold())),
        Line::from("    [↑ / ↓, j / k]   Navigate titles or sources"),
        Line::from("    [1 - 9]          Quick-select title or stream item directly"),
        Line::from("    [Tab / f]        Cycle Quality filter (All -> 4K -> 1080p -> 720p -> 480p)"),
        Line::from("    [p]              Cycle Provider filter (Torrentio, TPB, Torrents-CSV)"),
        Line::from("    [s]              Cycle Sort Order (By Seeders, Quality, or File Size)"),
        Line::from("    [n / N]          Binge-watching: Jump directly to Next Episode!"),
        Line::from("    [P]              Jump directly to Previous Episode"),
        Line::from("    [Enter]          Confirm title / Stream selected source in MPV"),
        Line::from("    [q / Esc]        Back to previous screen"),
        Line::from(""),
        Line::from(Span::styled("  --- Live TV (IPTV Mode) ---", Style::default().fg(Color::Yellow).bold())),
        Line::from("    [Tab / [ / ]]    Cycle category (News, Movies, Sports, Music, etc.)"),
        Line::from("    [Type]           Real-time channel name or country filter"),
        Line::from("    [Enter]          Play live channel in MPV"),
        Line::from(""),
        Line::from(Span::styled("  --- Press any key or [Esc] to dismiss help ---", Style::default().fg(Color::DarkGray).italic())),
    ];

    let p = Paragraph::new(help_text).block(block).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        let mut idx = max_len;
        while !s.is_char_boundary(idx) && idx > 0 {
            idx -= 1;
        }
        &s[..idx]
    }
}
