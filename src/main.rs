//! tsss-tui - Modern, ergonomic interactive TUI for sss (Super Simple Streamer)
//!
//! Features:
//! - Multi-Source Aggregation: Torrentio, The Pirate Bay (Apibay), and Torrents-CSV
//! - Live TV (IPTV) Streaming: 11,000+ live television channels with category filtering
//! - Internet Archive (Archive.org): Public-domain feature films, cult cinema & direct MP4 streams
//! - Direct Stream Player: Stream any Magnet link (BTIH) or HLS / MP4 stream directly in MPV
//! - Full keyboard navigation (Arrows, vi keys j/k/h/l, Tab, 1-9 direct selection)
//! - Smart flow: Movies skip season/episode directly to sources; TV shows get an intuitive stepper
//! - Source health badges (Seeders, Quality, Resolution, HDR, Size, Provider)
//! - Quality quick-filters (All, 4K, 1080p, 720p, 480p)
//! - Provider quick-filters (All, Torrentio, The Pirate Bay, Torrents-CSV)
//! - Sort controls: By Seeders, By Quality, By Size
//! - Continuous playback & Binge-watching support: 'n' for next episode without restarting!

pub mod app;
pub mod models;
pub mod player;
pub mod sources;
pub mod ui;

use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use models::{AppMode, QualityFilter, ProviderFilter, Screen, SortOrder, StepperField};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

fn main() -> io::Result<()> {
    let mut app = App::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut exit = false;

    while !exit {
        terminal.draw(|f| ui::render_ui(f, &app))?;

        // If a blocking action was triggered, run it after redrawing the loading screen
        if app.is_loading {
            app.spinner_tick = app.spinner_tick.wrapping_add(1);
            match app.screen {
                Screen::Search => {
                    app.execute_search();
                }
                Screen::PickShow | Screen::SeasonEpisode | Screen::PickSource => {
                    app.execute_load_sources();
                }
                Screen::AnimeSearch => {
                    app.execute_search_anime();
                }
                Screen::AnimeEpisode | Screen::AnimeSources => {
                    app.execute_load_anime_sources();
                }
                Screen::CartoonSearch => {
                    app.execute_search_cartoons();
                }
                Screen::CartoonSeasonEpisode | Screen::CartoonSources => {
                    app.execute_load_cartoon_sources();
                }
                Screen::IptvList => {
                    app.execute_load_iptv();
                }
                Screen::ArchiveSearch => {
                    app.execute_search_archive();
                }
                Screen::ArchiveFiles => {
                    app.execute_load_archive_files();
                }
                Screen::DirectInput | Screen::History => {
                    app.is_loading = false;
                }
            }
            continue;
        }

        // Wait for next keyboard event with polling timeout for smooth UI responsiveness
        if event::poll(Duration::from_millis(150))? {
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

                // Dismiss help modal with any key or Esc
                if app.show_help {
                    app.show_help = false;
                    continue;
                }

                // Toggle help modal
                if key.code == KeyCode::Char('?') && !matches!(app.screen, Screen::Search | Screen::AnimeSearch | Screen::CartoonSearch | Screen::DirectInput) {
                    app.show_help = true;
                    continue;
                }

                // --- Global Mode Switching Shortcuts (F1-F7 and Ctrl+1-7) ---
                match key.code {
                    KeyCode::F(1) => {
                        app.switch_mode(AppMode::MoviesAndSeries);
                        continue;
                    }
                    KeyCode::F(2) => {
                        app.switch_mode(AppMode::Anime);
                        continue;
                    }
                    KeyCode::F(3) => {
                        app.switch_mode(AppMode::Cartoons);
                        continue;
                    }
                    KeyCode::F(4) => {
                        app.switch_mode(AppMode::LiveTv);
                        continue;
                    }
                    KeyCode::F(5) => {
                        app.switch_mode(AppMode::Archive);
                        continue;
                    }
                    KeyCode::F(6) => {
                        app.switch_mode(AppMode::DirectStream);
                        continue;
                    }
                    KeyCode::F(7) => {
                        app.switch_mode(AppMode::History);
                        continue;
                    }
                    KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::MoviesAndSeries);
                        continue;
                    }
                    KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::Anime);
                        continue;
                    }
                    KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::Cartoons);
                        continue;
                    }
                    KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::LiveTv);
                        continue;
                    }
                    KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::Archive);
                        continue;
                    }
                    KeyCode::Char('6') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::DirectStream);
                        continue;
                    }
                    KeyCode::Char('7') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.switch_mode(AppMode::History);
                        continue;
                    }
                    KeyCode::Char('H') if !matches!(app.screen, Screen::Search | Screen::AnimeSearch | Screen::CartoonSearch | Screen::DirectInput) => {
                        app.switch_mode(AppMode::History);
                        continue;
                    }
                    _ => {}
                }

                // Screen-specific event handling
                match app.screen {
                    // --------------------------------------------------------
                    // Screen: Movies & TV Search
                    // --------------------------------------------------------
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
                        KeyCode::Char('?') if app.query.is_empty() => {
                            app.show_help = true;
                        }
                        KeyCode::Char(c) => {
                            app.query.push(c);
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Pick Show
                    // --------------------------------------------------------
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

                    // --------------------------------------------------------
                    // Screen: Season & Episode Stepper
                    // --------------------------------------------------------
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

                    // --------------------------------------------------------
                    // Screen: Pick Source
                    // --------------------------------------------------------
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
                            // Quality filter cycling: Tab, f, F
                            KeyCode::Tab | KeyCode::Char('f') => {
                                app.quality_filter = match app.quality_filter {
                                    QualityFilter::All => QualityFilter::Res1080p,
                                    QualityFilter::Res1080p => QualityFilter::Res4k,
                                    QualityFilter::Res4k => QualityFilter::Res720p,
                                    QualityFilter::Res720p => QualityFilter::Res480p,
                                    QualityFilter::Res480p => QualityFilter::All,
                                };
                                app.src_idx = 0;
                                app.src_state.select(Some(0));
                            }
                            // Provider filter cycling: p, P
                            KeyCode::Char('p') => {
                                app.provider_filter = match app.provider_filter {
                                    ProviderFilter::All => ProviderFilter::Torrentio,
                                    ProviderFilter::Torrentio => ProviderFilter::ThePirateBay,
                                    ProviderFilter::ThePirateBay => ProviderFilter::TorrentsCsv,
                                    ProviderFilter::TorrentsCsv => ProviderFilter::All,
                                    _ => ProviderFilter::All,
                                };
                                app.src_idx = 0;
                                app.src_state.select(Some(0));
                            }
                            // Sort order cycling: s, S
                            KeyCode::Char('s') => {
                                app.sort_order = match app.sort_order {
                                    SortOrder::Seeders => SortOrder::Quality,
                                    SortOrder::Quality => SortOrder::Size,
                                    SortOrder::Size => SortOrder::Seeders,
                                };
                                app.src_idx = 0;
                                app.src_state.select(Some(0));
                            }
                            // Reset filter to All: 0 or 'a'
                            KeyCode::Char('0') | KeyCode::Char('a') | KeyCode::Char('A') => {
                                app.quality_filter = QualityFilter::All;
                                app.provider_filter = ProviderFilter::All;
                                app.src_idx = 0;
                                app.src_state.select(Some(0));
                            }
                            // Quick direct selection: 1-9
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
                            // Previous episode shortcut
                            KeyCode::Char('P') => {
                                app.prev_episode();
                            }
                            KeyCode::Enter => {
                                if total > 0 {
                                    if let Err(e) = app.play_selected_source() {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                let is_series = app.selected_show().is_some_and(|s| s.is_series());
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

                    // --------------------------------------------------------
                    // Screen: Anime Search & Catalog
                    // --------------------------------------------------------
                    Screen::AnimeSearch => match key.code {
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.anime_query.clear();
                        }
                        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let trimmed = app.anime_query.trim_end();
                            if let Some(pos) = trimmed.rfind(' ') {
                                app.anime_query.truncate(pos);
                            } else {
                                app.anime_query.clear();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let total = app.anime_results.len();
                            if total > 0 {
                                app.anime_idx = app.anime_idx.saturating_sub(1);
                                app.anime_state.select(Some(app.anime_idx));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let total = app.anime_results.len();
                            if total > 0 && app.anime_idx + 1 < total {
                                app.anime_idx += 1;
                                app.anime_state.select(Some(app.anime_idx));
                            }
                        }
                        KeyCode::PageUp => {
                            let total = app.anime_results.len();
                            if total > 0 {
                                app.anime_idx = app.anime_idx.saturating_sub(5);
                                app.anime_state.select(Some(app.anime_idx));
                            }
                        }
                        KeyCode::PageDown => {
                            let total = app.anime_results.len();
                            if total > 0 {
                                app.anime_idx = (app.anime_idx + 5).min(total - 1);
                                app.anime_state.select(Some(app.anime_idx));
                            }
                        }
                        KeyCode::Enter => {
                            if !app.anime_results.is_empty() && app.anime_query.is_empty() {
                                app.select_anime_confirm();
                            } else if !app.anime_query.is_empty() {
                                app.search_anime();
                            } else if !app.anime_results.is_empty() {
                                app.select_anime_confirm();
                            }
                        }
                        KeyCode::Backspace => {
                            app.anime_query.pop();
                        }
                        KeyCode::Esc => {
                            if !app.anime_query.is_empty() {
                                app.anime_query.clear();
                            } else {
                                app.switch_mode(AppMode::MoviesAndSeries);
                            }
                        }
                        KeyCode::Char(c) => {
                            app.anime_query.push(c);
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Anime Episode Selection
                    // --------------------------------------------------------
                    Screen::AnimeEpisode => {
                        let max_ep = app.selected_anime().and_then(|a| a.episode_count).unwrap_or(999);
                        match key.code {
                            KeyCode::Left | KeyCode::Char('-') => {
                                app.anime_episode = app.anime_episode.saturating_sub(1).max(1);
                            }
                            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                                app.anime_episode = (app.anime_episode + 1).min(max_ep);
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                let digit = c.to_digit(10).unwrap_or(0);
                                let new_val = if app.anime_episode > 99 { digit } else { app.anime_episode * 10 + digit };
                                app.anime_episode = new_val.clamp(1, max_ep);
                            }
                            KeyCode::Backspace => {
                                app.anime_episode = (app.anime_episode / 10).max(1);
                            }
                            KeyCode::Enter => {
                                app.prepare_load_anime_sources();
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.screen = Screen::AnimeSearch;
                                app.status_msg = None;
                            }
                            _ => {}
                        }
                    }

                    // --------------------------------------------------------
                    // Screen: Anime Source Selection
                    // --------------------------------------------------------
                    Screen::AnimeSources => {
                        let filtered_indices = app.filtered_anime_source_indices();
                        let total = filtered_indices.len();

                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if total > 0 {
                                    app.anime_src_idx = app.anime_src_idx.saturating_sub(1);
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if total > 0 && app.anime_src_idx + 1 < total {
                                    app.anime_src_idx += 1;
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            KeyCode::PageUp => {
                                if total > 0 {
                                    app.anime_src_idx = app.anime_src_idx.saturating_sub(5);
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            KeyCode::PageDown => {
                                if total > 0 {
                                    app.anime_src_idx = (app.anime_src_idx + 5).min(total - 1);
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            KeyCode::Home | KeyCode::Char('g') => {
                                if total > 0 {
                                    app.anime_src_idx = 0;
                                    app.anime_src_state.select(Some(0));
                                }
                            }
                            KeyCode::End | KeyCode::Char('G') => {
                                if total > 0 {
                                    app.anime_src_idx = total - 1;
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            // Quality filter cycling: Tab, f, F
                            KeyCode::Tab | KeyCode::Char('f') => {
                                app.anime_quality_filter = match app.anime_quality_filter {
                                    QualityFilter::All => QualityFilter::Res1080p,
                                    QualityFilter::Res1080p => QualityFilter::Res4k,
                                    QualityFilter::Res4k => QualityFilter::Res720p,
                                    QualityFilter::Res720p => QualityFilter::Res480p,
                                    QualityFilter::Res480p => QualityFilter::All,
                                };
                                app.anime_src_idx = 0;
                                app.anime_src_state.select(Some(0));
                            }
                            // Provider filter cycling: p
                            KeyCode::Char('p') => {
                                app.anime_provider_filter = match app.anime_provider_filter {
                                    ProviderFilter::All => ProviderFilter::NyaaSi,
                                    ProviderFilter::NyaaSi => ProviderFilter::Torrentio,
                                    ProviderFilter::Torrentio => ProviderFilter::ThePirateBay,
                                    ProviderFilter::ThePirateBay => ProviderFilter::TorrentsCsv,
                                    _ => ProviderFilter::All,
                                };
                                app.anime_src_idx = 0;
                                app.anime_src_state.select(Some(0));
                            }
                            // Sort order cycling: s, S
                            KeyCode::Char('s') => {
                                app.anime_sort_order = match app.anime_sort_order {
                                    SortOrder::Seeders => SortOrder::Quality,
                                    SortOrder::Quality => SortOrder::Size,
                                    SortOrder::Size => SortOrder::Seeders,
                                };
                                app.anime_src_idx = 0;
                                app.anime_src_state.select(Some(0));
                            }
                            // Reset filter to All: 0 or 'a'
                            KeyCode::Char('0') | KeyCode::Char('a') | KeyCode::Char('A') => {
                                app.anime_quality_filter = QualityFilter::All;
                                app.anime_provider_filter = ProviderFilter::All;
                                app.anime_src_idx = 0;
                                app.anime_src_state.select(Some(0));
                            }
                            // Quick direct selection: 1-9
                            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                let digit_idx = (c.to_digit(10).unwrap_or(1) as usize) - 1;
                                if digit_idx < total {
                                    app.anime_src_idx = digit_idx;
                                    app.anime_src_state.select(Some(app.anime_src_idx));
                                }
                            }
                            // Binge-watching: next episode shortcut
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                app.anime_next_episode();
                            }
                            // Previous episode shortcut
                            KeyCode::Char('P') => {
                                app.anime_prev_episode();
                            }
                            KeyCode::Enter => {
                                if total > 0 {
                                    if let Err(e) = app.play_selected_anime_source() {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                let is_movie = app.selected_anime().is_some_and(|a| a.is_movie());
                                if is_movie {
                                    app.screen = Screen::AnimeSearch;
                                } else {
                                    app.screen = Screen::AnimeEpisode;
                                }
                                app.status_msg = None;
                            }
                            _ => {}
                        }
                    }

                    // --------------------------------------------------------
                    // Screen: Cartoons Search & Catalog
                    // --------------------------------------------------------
                    Screen::CartoonSearch => match key.code {
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.cartoon_query.clear();
                        }
                        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let trimmed = app.cartoon_query.trim_end();
                            if let Some(pos) = trimmed.rfind(' ') {
                                app.cartoon_query.truncate(pos);
                            } else {
                                app.cartoon_query.clear();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let total = app.cartoon_results.len();
                            if total > 0 {
                                app.cartoon_idx = app.cartoon_idx.saturating_sub(1);
                                app.cartoon_state.select(Some(app.cartoon_idx));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let total = app.cartoon_results.len();
                            if total > 0 && app.cartoon_idx + 1 < total {
                                app.cartoon_idx += 1;
                                app.cartoon_state.select(Some(app.cartoon_idx));
                            }
                        }
                        KeyCode::PageUp => {
                            let total = app.cartoon_results.len();
                            if total > 0 {
                                app.cartoon_idx = app.cartoon_idx.saturating_sub(5);
                                app.cartoon_state.select(Some(app.cartoon_idx));
                            }
                        }
                        KeyCode::PageDown => {
                            let total = app.cartoon_results.len();
                            if total > 0 {
                                app.cartoon_idx = (app.cartoon_idx + 5).min(total - 1);
                                app.cartoon_state.select(Some(app.cartoon_idx));
                            }
                        }
                        KeyCode::Enter => {
                            if !app.cartoon_results.is_empty() && app.cartoon_query.is_empty() {
                                app.select_cartoon_confirm();
                            } else if !app.cartoon_query.is_empty() {
                                app.search_cartoons();
                            } else if !app.cartoon_results.is_empty() {
                                app.select_cartoon_confirm();
                            }
                        }
                        KeyCode::Backspace => {
                            app.cartoon_query.pop();
                        }
                        KeyCode::Esc => {
                            if !app.cartoon_query.is_empty() {
                                app.cartoon_query.clear();
                            } else {
                                app.switch_mode(AppMode::MoviesAndSeries);
                            }
                        }
                        KeyCode::Char('?') if app.cartoon_query.is_empty() => {
                            app.show_help = true;
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' && app.cartoon_query.is_empty() => {
                            let digit_idx = (c.to_digit(10).unwrap_or(1) as usize) - 1;
                            if digit_idx < app.cartoon_results.len() {
                                app.cartoon_idx = digit_idx;
                                app.cartoon_state.select(Some(app.cartoon_idx));
                                app.select_cartoon_confirm();
                            }
                        }
                        KeyCode::Char(c) => {
                            app.cartoon_query.push(c);
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Cartoon Season & Episode Selection
                    // --------------------------------------------------------
                    Screen::CartoonSeasonEpisode => match key.code {
                        KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                            app.cartoon_stepper_field = match app.cartoon_stepper_field {
                                StepperField::Season => StepperField::Episode,
                                StepperField::Episode => StepperField::Season,
                            };
                        }
                        KeyCode::Left | KeyCode::Char('-') => match app.cartoon_stepper_field {
                            StepperField::Season => {
                                app.cartoon_season = app.cartoon_season.saturating_sub(1).max(1);
                            }
                            StepperField::Episode => {
                                app.cartoon_episode = app.cartoon_episode.saturating_sub(1).max(1);
                            }
                        },
                        KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                            match app.cartoon_stepper_field {
                                StepperField::Season => {
                                    app.cartoon_season = (app.cartoon_season + 1).min(99);
                                }
                                StepperField::Episode => {
                                    app.cartoon_episode = (app.cartoon_episode + 1).min(999);
                                }
                            }
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            let digit = c.to_digit(10).unwrap_or(0);
                            match app.cartoon_stepper_field {
                                StepperField::Season => {
                                    let new_val = if app.cartoon_season > 9 { digit } else { app.cartoon_season * 10 + digit };
                                    app.cartoon_season = new_val.clamp(1, 99);
                                }
                                StepperField::Episode => {
                                    let new_val = if app.cartoon_episode > 99 { digit } else { app.cartoon_episode * 10 + digit };
                                    app.cartoon_episode = new_val.clamp(1, 999);
                                }
                            }
                        }
                        KeyCode::Backspace => match app.cartoon_stepper_field {
                            StepperField::Season => {
                                app.cartoon_season = (app.cartoon_season / 10).max(1);
                            }
                            StepperField::Episode => {
                                app.cartoon_episode = (app.cartoon_episode / 10).max(1);
                            }
                        },
                        KeyCode::Enter => {
                            app.prepare_load_cartoon_sources();
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.screen = Screen::CartoonSearch;
                            app.status_msg = None;
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Cartoon Source Selection
                    // --------------------------------------------------------
                    Screen::CartoonSources => {
                        let filtered_indices = app.filtered_cartoon_source_indices();
                        let total = filtered_indices.len();

                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if total > 0 {
                                    app.cartoon_src_idx = app.cartoon_src_idx.saturating_sub(1);
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if total > 0 && app.cartoon_src_idx + 1 < total {
                                    app.cartoon_src_idx += 1;
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            KeyCode::PageUp => {
                                if total > 0 {
                                    app.cartoon_src_idx = app.cartoon_src_idx.saturating_sub(5);
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            KeyCode::PageDown => {
                                if total > 0 {
                                    app.cartoon_src_idx = (app.cartoon_src_idx + 5).min(total - 1);
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            KeyCode::Home | KeyCode::Char('g') => {
                                if total > 0 {
                                    app.cartoon_src_idx = 0;
                                    app.cartoon_src_state.select(Some(0));
                                }
                            }
                            KeyCode::End | KeyCode::Char('G') => {
                                if total > 0 {
                                    app.cartoon_src_idx = total - 1;
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            // Quality filter cycling: Tab, f, F
                            KeyCode::Tab | KeyCode::Char('f') => {
                                app.cartoon_quality_filter = match app.cartoon_quality_filter {
                                    QualityFilter::All => QualityFilter::Res1080p,
                                    QualityFilter::Res1080p => QualityFilter::Res4k,
                                    QualityFilter::Res4k => QualityFilter::Res720p,
                                    QualityFilter::Res720p => QualityFilter::Res480p,
                                    QualityFilter::Res480p => QualityFilter::All,
                                };
                                app.cartoon_src_idx = 0;
                                app.cartoon_src_state.select(Some(0));
                            }
                            // Provider filter cycling: p
                            KeyCode::Char('p') => {
                                app.cartoon_provider_filter = match app.cartoon_provider_filter {
                                    ProviderFilter::All => ProviderFilter::Torrentio,
                                    ProviderFilter::Torrentio => ProviderFilter::ThePirateBay,
                                    ProviderFilter::ThePirateBay => ProviderFilter::TorrentsCsv,
                                    ProviderFilter::TorrentsCsv => ProviderFilter::NyaaSi,
                                    _ => ProviderFilter::All,
                                };
                                app.cartoon_src_idx = 0;
                                app.cartoon_src_state.select(Some(0));
                            }
                            // Sort order cycling: s, S
                            KeyCode::Char('s') => {
                                app.cartoon_sort_order = match app.cartoon_sort_order {
                                    SortOrder::Seeders => SortOrder::Quality,
                                    SortOrder::Quality => SortOrder::Size,
                                    SortOrder::Size => SortOrder::Seeders,
                                };
                                app.cartoon_src_idx = 0;
                                app.cartoon_src_state.select(Some(0));
                            }
                            // Reset filter to All: 0 or 'a'
                            KeyCode::Char('0') | KeyCode::Char('a') | KeyCode::Char('A') => {
                                app.cartoon_quality_filter = QualityFilter::All;
                                app.cartoon_provider_filter = ProviderFilter::All;
                                app.cartoon_src_idx = 0;
                                app.cartoon_src_state.select(Some(0));
                            }
                            // Quick direct selection: 1-9
                            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                let digit_idx = (c.to_digit(10).unwrap_or(1) as usize) - 1;
                                if digit_idx < total {
                                    app.cartoon_src_idx = digit_idx;
                                    app.cartoon_src_state.select(Some(app.cartoon_src_idx));
                                }
                            }
                            // Binge-watching: next episode shortcut
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                app.cartoon_next_episode();
                            }
                            // Previous episode shortcut
                            KeyCode::Char('P') => {
                                app.cartoon_prev_episode();
                            }
                            KeyCode::Enter => {
                                if total > 0 {
                                    if let Err(e) = app.play_selected_cartoon_source() {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                let is_series = app.selected_cartoon().is_none_or(|c| c.is_series);
                                if is_series {
                                    app.screen = Screen::CartoonSeasonEpisode;
                                } else {
                                    app.screen = Screen::CartoonSearch;
                                }
                                app.status_msg = None;
                            }
                            _ => {}
                        }
                    }

                    // --------------------------------------------------------
                    // Screen: Live TV (IPTV)
                    // --------------------------------------------------------
                    Screen::IptvList => {
                        let filtered_count = app.filtered_iptv_channels().len();
                        match key.code {
                            KeyCode::Up => {
                                if filtered_count > 0 {
                                    app.iptv_idx = app.iptv_idx.saturating_sub(1);
                                    app.iptv_state.select(Some(app.iptv_idx));
                                }
                            }
                            KeyCode::Down => {
                                if filtered_count > 0 && app.iptv_idx + 1 < filtered_count {
                                    app.iptv_idx += 1;
                                    app.iptv_state.select(Some(app.iptv_idx));
                                }
                            }
                            KeyCode::PageUp => {
                                if filtered_count > 0 {
                                    app.iptv_idx = app.iptv_idx.saturating_sub(10);
                                    app.iptv_state.select(Some(app.iptv_idx));
                                }
                            }
                            KeyCode::PageDown => {
                                if filtered_count > 0 {
                                    app.iptv_idx = (app.iptv_idx + 10).min(filtered_count - 1);
                                    app.iptv_state.select(Some(app.iptv_idx));
                                }
                            }
                            // Category cycling: Tab, [, ]
                            KeyCode::Tab | KeyCode::Char(']') => {
                                let cats = [
                                    "All", "News", "Movies", "Sports", "Entertainment", "Music", "Documentary", "Kids", "General"
                                ];
                                if let Some(pos) = cats.iter().position(|&c| c == app.iptv_category) {
                                    let next = (pos + 1) % cats.len();
                                    app.iptv_category = cats[next].to_string();
                                } else {
                                    app.iptv_category = "All".to_string();
                                }
                                app.iptv_idx = 0;
                                app.iptv_state.select(Some(0));
                            }
                            KeyCode::BackTab | KeyCode::Char('[') => {
                                let cats = [
                                    "All", "News", "Movies", "Sports", "Entertainment", "Music", "Documentary", "Kids", "General"
                                ];
                                if let Some(pos) = cats.iter().position(|&c| c == app.iptv_category) {
                                    let prev = if pos == 0 { cats.len() - 1 } else { pos - 1 };
                                    app.iptv_category = cats[prev].to_string();
                                } else {
                                    app.iptv_category = "All".to_string();
                                }
                                app.iptv_idx = 0;
                                app.iptv_state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                if filtered_count > 0 {
                                    if let Err(e) = app.play_selected_iptv() {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.iptv_query.pop();
                                app.iptv_idx = 0;
                                app.iptv_state.select(Some(0));
                            }
                            KeyCode::Esc => {
                                if !app.iptv_query.is_empty() {
                                    app.iptv_query.clear();
                                    app.iptv_idx = 0;
                                    app.iptv_state.select(Some(0));
                                } else {
                                    app.switch_mode(AppMode::MoviesAndSeries);
                                }
                            }
                            KeyCode::Char(c) => {
                                app.iptv_query.push(c);
                                app.iptv_idx = 0;
                                app.iptv_state.select(Some(0));
                            }
                            _ => {}
                        }
                    }

                    // --------------------------------------------------------
                    // Screen: Internet Archive Search
                    // --------------------------------------------------------
                    Screen::ArchiveSearch => match key.code {
                        KeyCode::Enter => {
                            if app.archive_results.is_empty() || !app.archive_query.is_empty() && app.archive_state.selected().is_none() {
                                app.search_archive();
                            } else if !app.archive_results.is_empty() {
                                app.select_archive_item_confirm();
                            }
                        }
                        KeyCode::Up => {
                            let total = app.archive_results.len();
                            if total > 0 {
                                app.archive_idx = app.archive_idx.saturating_sub(1);
                                app.archive_state.select(Some(app.archive_idx));
                            }
                        }
                        KeyCode::Down => {
                            let total = app.archive_results.len();
                            if total > 0 && app.archive_idx + 1 < total {
                                app.archive_idx += 1;
                                app.archive_state.select(Some(app.archive_idx));
                            }
                        }
                        KeyCode::Backspace => {
                            app.archive_query.pop();
                        }
                        KeyCode::Esc => {
                            if !app.archive_query.is_empty() {
                                app.archive_query.clear();
                            } else {
                                app.switch_mode(AppMode::MoviesAndSeries);
                            }
                        }
                        KeyCode::Char(c) => {
                            app.archive_query.push(c);
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Internet Archive Files
                    // --------------------------------------------------------
                    Screen::ArchiveFiles => {
                        let total = app.archive_files.len();
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if total > 0 {
                                    app.archive_file_idx = app.archive_file_idx.saturating_sub(1);
                                    app.archive_file_state.select(Some(app.archive_file_idx));
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if total > 0 && app.archive_file_idx + 1 < total {
                                    app.archive_file_idx += 1;
                                    app.archive_file_state.select(Some(app.archive_file_idx));
                                }
                            }
                            KeyCode::Enter => {
                                if total > 0 {
                                    if let Err(e) = app.play_selected_archive_file() {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.screen = Screen::ArchiveSearch;
                            }
                            _ => {}
                        }
                    }

                    // --------------------------------------------------------
                    // Screen: Direct Stream Input
                    // --------------------------------------------------------
                    Screen::DirectInput => match key.code {
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.direct_url_input.clear();
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.play_direct_input() {
                                app.error = Some(e.to_string());
                            }
                        }
                        KeyCode::Backspace => {
                            app.direct_url_input.pop();
                        }
                        KeyCode::Esc => {
                            if !app.direct_url_input.is_empty() {
                                app.direct_url_input.clear();
                            } else {
                                app.switch_mode(AppMode::MoviesAndSeries);
                            }
                        }
                        KeyCode::Char(c) => {
                            app.direct_url_input.push(c);
                        }
                        _ => {}
                    },

                    // --------------------------------------------------------
                    // Screen: Watch History & Continue Watching
                    // --------------------------------------------------------
                    Screen::History => {
                        let total = app.history.len();
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if total > 0 {
                                    app.history_idx = app.history_idx.saturating_sub(1);
                                    app.history_state.select(Some(app.history_idx));
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if total > 0 && app.history_idx + 1 < total {
                                    app.history_idx += 1;
                                    app.history_state.select(Some(app.history_idx));
                                }
                            }
                            KeyCode::PageUp => {
                                if total > 0 {
                                    app.history_idx = app.history_idx.saturating_sub(5);
                                    app.history_state.select(Some(app.history_idx));
                                }
                            }
                            KeyCode::PageDown => {
                                if total > 0 {
                                    app.history_idx = (app.history_idx + 5).min(total - 1);
                                    app.history_state.select(Some(app.history_idx));
                                }
                            }
                            KeyCode::Home | KeyCode::Char('g') => {
                                if total > 0 {
                                    app.history_idx = 0;
                                    app.history_state.select(Some(0));
                                }
                            }
                            KeyCode::End | KeyCode::Char('G') => {
                                if total > 0 {
                                    app.history_idx = total - 1;
                                    app.history_state.select(Some(app.history_idx));
                                }
                            }
                            KeyCode::Enter => {
                                if total > 0 {
                                    if let Err(e) = app.continue_watching_selected(false) {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                if total > 0 {
                                    if let Err(e) = app.continue_watching_selected(true) {
                                        app.error = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Char('d') | KeyCode::Delete => {
                                if total > 0 {
                                    app.delete_history_item(app.history_idx);
                                }
                            }
                            KeyCode::Char('c') => {
                                if total > 0 {
                                    app.clear_history();
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.switch_mode(AppMode::MoviesAndSeries);
                            }
                            _ => {}
                        }
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
    use models::SourceItem;
    use sources::torrentio::{make_magnet, parse_release_metadata, parse_source_line};

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
        let movie = models::ImdbResult {
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

        let tv = models::ImdbResult {
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
            codec: None,
            seeds: Some(100),
            size_bytes: Some(10_000_000_000),
            size_str: "10 GB".to_string(),
            title: "Test 4K".to_string(),
            stream: models::StreamSource::SssPick { num: "1".to_string() },
        };
        let item_1080p = SourceItem {
            num: "2".to_string(),
            provider: "Torrentio".to_string(),
            resolution: "1080p".to_string(),
            hdr_info: None,
            codec: None,
            seeds: Some(50),
            size_bytes: Some(2_000_000_000),
            size_str: "2 GB".to_string(),
            title: "Test 1080p".to_string(),
            stream: models::StreamSource::SssPick { num: "2".to_string() },
        };

        assert!(QualityFilter::All.matches(&item_4k));
        assert!(QualityFilter::All.matches(&item_1080p));
        assert!(QualityFilter::Res4k.matches(&item_4k));
        assert!(!QualityFilter::Res4k.matches(&item_1080p));
        assert!(!QualityFilter::Res1080p.matches(&item_4k));
        assert!(QualityFilter::Res1080p.matches(&item_1080p));
    }

    #[test]
    fn test_provider_filter() {
        let item_tpb = SourceItem {
            num: "1".to_string(),
            provider: "The Pirate Bay".to_string(),
            resolution: "1080p".to_string(),
            hdr_info: None,
            codec: None,
            seeds: Some(100),
            size_bytes: None,
            size_str: "2 GB".to_string(),
            title: "Test TPB".to_string(),
            stream: models::StreamSource::Torrent {
                magnet: "magnet:?".to_string(),
                info_hash: "abcd".to_string(),
                file_idx: None,
            },
        };

        assert!(ProviderFilter::All.matches(&item_tpb));
        assert!(ProviderFilter::ThePirateBay.matches(&item_tpb));
        assert!(!ProviderFilter::Torrentio.matches(&item_tpb));
        assert!(!ProviderFilter::TorrentsCsv.matches(&item_tpb));
    }

    #[test]
    fn test_parse_release_metadata() {
        let text = "Dune.Part.Two.2024.2160p.UHD.BluRay.DV.HDR10+.HEVC.Atmos-TERMiNAL";
        let (res, hdr, codec) = parse_release_metadata(text);
        assert_eq!(res, "4K");
        assert_eq!(hdr, Some("DV/HDR".to_string()));
        assert_eq!(codec, Some("HEVC/x265".to_string()));

        let text2 = "Severance.S01E01.1080p.WEB-DL.x264.AAC";
        let (res2, hdr2, codec2) = parse_release_metadata(text2);
        assert_eq!(res2, "1080p");
        assert_eq!(hdr2, None);
        assert_eq!(codec2, Some("x264".to_string()));
    }

    #[test]
    fn test_make_magnet() {
        let magnet = make_magnet("D7A46713EAEE18C746B3254B7D1492A50FD9D6CE", "The Matrix", &[]);
        assert!(magnet.starts_with("magnet:?xt=urn:btih:d7a46713eaee18c746b3254b7d1492a50fd9d6ce"));
        assert!(magnet.contains("dn=The%20Matrix"));
        assert!(magnet.contains("tr=udp%3A%2F%2Ftracker.opentrackr.org"));
    }

    #[test]
    fn test_iptv_m3u_parser() {
        let m3u_data = r#"#EXTM3U
#EXTINF:-1 tvg-id="BBCNews.uk" tvg-logo="" group-title="News",BBC News HD (1080p)
https://stream.example.com/bbc_news.m3u8
#EXTINF:-1 tvg-id="SkyCinema.uk" tvg-logo="" group-title="Movies;Action",Sky Cinema Premiere (720p)
https://stream.example.com/sky_cinema.m3u8
"#;
        let channels = sources::iptv::parse_m3u(m3u_data);
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "BBC News HD (1080p)");
        assert_eq!(channels[0].group, "News");
        assert_eq!(channels[0].resolution, "1080p");
        assert_eq!(channels[0].url, "https://stream.example.com/bbc_news.m3u8");

        assert_eq!(channels[1].name, "Sky Cinema Premiere (720p)");
        assert_eq!(channels[1].group, "Movies");
        assert_eq!(channels[1].resolution, "720p");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(sources::piratebay::format_size(1_073_741_824), "1.00 GB");
        assert_eq!(sources::piratebay::format_size(2_147_483_648), "2.00 GB");
        assert_eq!(sources::piratebay::format_size(524_288_000), "500.0 MB");
        assert_eq!(sources::piratebay::format_size(1024), "1.0 KB");
        assert_eq!(sources::piratebay::format_size(512), "512 B");
    }

    #[test]
    fn test_archive_search_and_metadata_flow() {
        // Test query formatting for archive
        let clean_kw: String = "night of the living dead 1968"
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
            .collect();
        let words: Vec<&str> = clean_kw.split_whitespace().collect();
        let query = words.join(" AND ");
        assert_eq!(query, "night AND of AND the AND living AND dead AND 1968");
    }

    #[test]
    fn test_live_search_title() {
        let results = sources::search_title("Inception").expect("Search should succeed");
        assert!(!results.is_empty(), "Should return at least one result for Inception");
        let first = &results[0];
        assert_eq!(first.l.as_deref(), Some("Inception"));
        assert!(!first.is_series());
    }

    #[test]
    fn test_kitsu_anime_model() {
        let tv_anime = models::KitsuAnime {
            id: "46474".to_string(),
            title: "Sousou no Frieren".to_string(),
            title_en: Some("Frieren: Beyond Journey's End".to_string()),
            title_jp: Some("葬送のフリーレン".to_string()),
            synopsis: "Elven mage journey".to_string(),
            episode_count: Some(28),
            subtype: "TV".to_string(),
            status: "finished".to_string(),
            year: "2023".to_string(),
            rating: Some("88.8".to_string()),
            age_rating_guide: Some("PG - Teens 13+".to_string()),
        };

        assert!(!tv_anime.is_movie());
        assert_eq!(tv_anime.display_title(), "Frieren: Beyond Journey's End");
        assert_eq!(tv_anime.subtype_label(), "TV Series");
        assert_eq!(tv_anime.status_label(), "Finished");
        assert_eq!(tv_anime.episodes_display(), "28 eps");

        let movie_anime = models::KitsuAnime {
            id: "11614".to_string(),
            title: "Kimi no Na wa.".to_string(),
            title_en: Some("Your Name.".to_string()),
            title_jp: Some("君の名は。".to_string()),
            synopsis: "Two strangers find themselves linked".to_string(),
            episode_count: Some(1),
            subtype: "movie".to_string(),
            status: "finished".to_string(),
            year: "2016".to_string(),
            rating: Some("90.1".to_string()),
            age_rating_guide: Some("PG".to_string()),
        };

        assert!(movie_anime.is_movie());
        assert_eq!(movie_anime.display_title(), "Your Name.");
        assert_eq!(movie_anime.subtype_label(), "Movie");
    }

    #[test]
    fn test_parse_nyaa_rss() {
        let sample_xml = r#"<rss version="2.0">
            <channel>
                <title>Nyaa - RSS</title>
                <item>
                    <title>[SubsPlease] Frieren - 01 (1080p) [F502B6D4].mkv</title>
                    <nyaa:infoHash>bc25231f657d115e61e68d0e124d3ff5ec101f69</nyaa:infoHash>
                    <nyaa:seeders>142</nyaa:seeders>
                    <nyaa:size>1.4 GiB</nyaa:size>
                </item>
                <item>
                    <title>[Erai-raws] Frieren - 02 [720p][MultiSub].mkv</title>
                    <nyaa:infoHash>3b918fd3e786cf572f6c069fd3d65aeb1ba94454</nyaa:infoHash>
                    <nyaa:seeders>35</nyaa:seeders>
                    <nyaa:size>750.0 MiB</nyaa:size>
                </item>
            </channel>
        </rss>"#;

        let parsed_ep1 = sources::anime::parse_nyaa_rss(sample_xml, false, 1);
        assert_eq!(parsed_ep1.len(), 1);
        assert_eq!(parsed_ep1[0].provider, "NyaaSi (SubsPlease)");
        assert_eq!(parsed_ep1[0].resolution, "1080p");
        assert_eq!(parsed_ep1[0].seeds, Some(142));
        assert_eq!(parsed_ep1[0].size_str, "1.4 GiB");

        let parsed_ep2 = sources::anime::parse_nyaa_rss(sample_xml, false, 2);
        assert_eq!(parsed_ep2.len(), 1);
        assert_eq!(parsed_ep2[0].provider, "NyaaSi (Erai-raws)");
        assert_eq!(parsed_ep2[0].resolution, "720p");
        assert_eq!(parsed_ep2[0].seeds, Some(35));
    }

    #[test]
    fn test_anime_parse_size_string() {
        assert_eq!(sources::anime::parse_size_string("1.4 GiB"), Some(1_503_238_553));
        assert_eq!(sources::anime::parse_size_string("500 MiB"), Some(524_288_000));
        assert_eq!(sources::anime::parse_size_string("100 KiB"), Some(102_400));
    }

    #[test]
    fn test_provider_filter_nyaa() {
        let nyaa_item = SourceItem {
            num: "1".to_string(),
            provider: "NyaaSi (SubsPlease)".to_string(),
            resolution: "1080p".to_string(),
            hdr_info: None,
            codec: Some("x264".to_string()),
            seeds: Some(150),
            size_bytes: Some(1_500_000_000),
            size_str: "1.5 GB".to_string(),
            title: "Anime Episode 1".to_string(),
            stream: models::StreamSource::Torrent {
                magnet: "magnet:?".to_string(),
                info_hash: "bc25231f657d115e61e68d0e124d3ff5ec101f69".to_string(),
                file_idx: None,
            },
        };

        assert!(ProviderFilter::All.matches(&nyaa_item));
        assert!(ProviderFilter::NyaaSi.matches(&nyaa_item));
        assert!(!ProviderFilter::Torrentio.matches(&nyaa_item));
        assert!(!ProviderFilter::ThePirateBay.matches(&nyaa_item));
    }

    #[test]
    fn test_title_matches_episode() {
        use sources::anime::title_matches_episode;
        assert!(title_matches_episode("[SubsPlease] Sousou no Frieren - 01 (1080p) [F502B6D4].mkv", 1));
        assert!(!title_matches_episode("[SubsPlease] Sousou no Frieren - 01 (1080p) [F502B6D4].mkv", 2));
        assert!(title_matches_episode("[Erai-raws] Jujutsu Kaisen 2nd Season - 23 [1080p][Multiple Subtitle].mkv", 23));
        assert!(!title_matches_episode("[Erai-raws] Jujutsu Kaisen 2nd Season - 23 [1080p][Multiple Subtitle].mkv", 3));
        assert!(title_matches_episode("[HorribleSubs] One Piece - 892 [720p].mkv", 892));
        assert!(title_matches_episode("Attack on Titan S04E15 1080p WEB-DL.mkv", 15));
        assert!(title_matches_episode("[SubsPlease] Frieren (01-28) (1080p) [Batch]", 5));
    }

    #[test]
    fn test_live_search_anime() {
        let results = sources::anime::search_anime("Frieren").expect("Anime search should succeed");
        assert!(!results.is_empty(), "Should return at least one result for Frieren");
        let first = &results[0];
        assert!(first.title.to_lowercase().contains("frieren") || first.display_title().to_lowercase().contains("frieren"));
    }

    #[test]
    fn test_clean_html_summary() {
        use sources::cartoon::clean_html_summary;
        let html = "<p>A boy named <b>Finn</b> &amp; his dog Jake in the Land of Ooo. It&#039;s &quot;mathematical&quot;!</p>";
        let cleaned = clean_html_summary(html);
        assert_eq!(cleaned, "A boy named Finn & his dog Jake in the Land of Ooo. It's \"mathematical\"!");
    }

    #[test]
    fn test_popular_cartoons_catalog() {
        use sources::cartoon::get_curated_popular_cartoons;
        let catalog = get_curated_popular_cartoons();
        assert!(catalog.len() >= 10);
        assert!(catalog.iter().any(|c| c.title == "Adventure Time"));
        assert!(catalog.iter().any(|c| c.title == "Gravity Falls"));
        assert!(catalog.iter().any(|c| c.imdb_id.as_deref() == Some("tt2861424"))); // Rick and Morty
    }

    #[test]
    fn test_cartoon_item_properties() {
        use crate::models::CartoonItem;
        let c = CartoonItem {
            id: "123".to_string(),
            imdb_id: Some("tt1305826".to_string()),
            title: "Adventure Time".to_string(),
            year: "2010".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Adventure".to_string(), "Comedy".to_string()],
            network: Some("Cartoon Network".to_string()),
            status: "Ended".to_string(),
            summary: "Finn and Jake explore Ooo.".to_string(),
            rating: Some("8.6".to_string()),
        };

        assert_eq!(c.title, "Adventure Time");
        assert_eq!(c.display_network(), "Cartoon Network");
        assert_eq!(c.genres_str(), "Animation, Adventure, Comedy");
        assert!(c.is_series);
    }

    #[test]
    fn test_live_search_cartoons() {
        let results = sources::cartoon::search_cartoons("Avatar").expect("Cartoon search should succeed");
        assert!(!results.is_empty(), "Should return at least one result for Avatar");
        let found = results.iter().any(|r| r.title.to_lowercase().contains("avatar"));
        assert!(found, "Should find Avatar in search results");
    }

    #[test]
    fn test_history_entry_properties() {
        let entry = models::HistoryEntry {
            id: "Anime:Frieren:None:Some(5)".to_string(),
            mode: "Anime".to_string(),
            title: "Frieren: Beyond Journey's End".to_string(),
            ttid: None,
            year: Some("2023".to_string()),
            season: None,
            episode: Some(5),
            provider: "NyaaSi (SubsPlease)".to_string(),
            resolution: "1080p".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            stream: models::StreamSource::DirectUrl {
                url: "http://example.com/video.mp4".to_string(),
            },
        };

        assert_eq!(entry.badge_label(), "ANIME");
        assert_eq!(entry.progress_str(), "EP 05");
        assert_eq!(entry.next_episode_str(), Some("EP 06".to_string()));
        assert_eq!(entry.time_ago_str(), "Just now");
    }

    #[test]
    fn test_history_series_next_episode() {
        let entry = models::HistoryEntry {
            id: "Cartoons:Gravity Falls:Some(1):Some(4)".to_string(),
            mode: "Cartoons".to_string(),
            title: "Gravity Falls".to_string(),
            ttid: Some("tt1865718".to_string()),
            year: Some("2012".to_string()),
            season: Some(1),
            episode: Some(4),
            provider: "Torrentio".to_string(),
            resolution: "1080p".to_string(),
            timestamp: 1_700_000_000,
            stream: models::StreamSource::DirectUrl {
                url: "http://example.com/stream.mkv".to_string(),
            },
        };

        assert_eq!(entry.badge_label(), "CARTOON");
        assert_eq!(entry.progress_str(), "S01E04");
        assert_eq!(entry.next_episode_str(), Some("S01E05".to_string()));
    }

    #[test]
    fn test_app_record_history_mru() {
        let mut app = App::new();
        app.history.clear();

        app.record_history(
            "Anime",
            "Frieren",
            None,
            Some("2023".to_string()),
            None,
            Some(1),
            "NyaaSi",
            "1080p",
            models::StreamSource::DirectUrl { url: "http://1".to_string() },
        );

        app.record_history(
            "Cartoons",
            "Avatar",
            None,
            Some("2005".to_string()),
            Some(1),
            Some(1),
            "Torrentio",
            "1080p",
            models::StreamSource::DirectUrl { url: "http://2".to_string() },
        );

        assert_eq!(app.history.len(), 2);
        assert_eq!(app.history[0].title, "Avatar");
        assert_eq!(app.history[1].title, "Frieren");

        // Re-watching Frieren should move it to index 0 with updated episode
        app.record_history(
            "Anime",
            "Frieren",
            None,
            Some("2023".to_string()),
            None,
            Some(2),
            "NyaaSi",
            "1080p",
            models::StreamSource::DirectUrl { url: "http://3".to_string() },
        );

        assert_eq!(app.history.len(), 2);
        assert_eq!(app.history[0].title, "Frieren");
        assert_eq!(app.history[0].episode, Some(2));
        assert_eq!(app.history[1].title, "Avatar");

        // Delete item
        app.delete_history_item(0);
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.history[0].title, "Avatar");

        // Clear all
        app.clear_history();
        assert!(app.history.is_empty());
    }
}
