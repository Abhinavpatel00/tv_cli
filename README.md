# 🎬 tsss-tui

> Modern, ultra-fast, interactive Terminal User Interface (TUI) for streaming movies, TV series, anime, live TV, and public-domain cinema directly in MPV without ads, tracking, or browser overhead.

Built in **Rust** using **Ratatui** and **Crossterm**.

---

## 🌟 What's New & Supported Streaming Data Sources

`tsss-tui` aggregates multiple high-speed media indexers and streaming protocols:

### 1. ⚡ Multi-Source Torrent Indexing (Movies & TV Series)
Simultaneously queries and merges results in parallel across top indexers:
- **Torrentio**: Scrapes 25+ major providers (YTS, EZTV, 1337x, RARBG, TorrentGalaxy, MagnetDL, Knaben, etc.).
- **The Pirate Bay (Apibay)**: Direct real-time queries for movies and series with live seeder tracking.
- **Torrents-CSV**: Decentralized, open-source BitTorrent database with over 35 million indexed records.
- **Intelligent Deduplication & Swarm Merging**: Matches identical infohashes, pools public trackers, and selects the healthiest peer swarm with maximum seeders.

### 2. ⛩ Anime Streaming (Kitsu Catalog • Nyaa.si • Torrentio Anime)
Dedicated first-class Anime streaming mode:
- **Curated Catalog & Search**: Auto-loads trending & popular anime (Attack on Titan, Frieren, Demon Slayer, Jujutsu Kaisen, One Piece, etc.) or searches Kitsu API with synopses, Japanese/Romaji titles, episode counts, community ratings, and airing status.
- **Multi-Source Anime Aggregation**: Queries **Nyaa.si** (direct live RSS feed scraper for the latest simulcasts), **Torrentio Anime** (indexing NyaaSi, AniDex, TokyoTosho, AnimeTosho, HorribleSubs, Rutracker), and P2P indexers in parallel.
- **Soft Subtitles & Dual Audio**: Leverages embedded ASS/SSA styled soft-subtitles and multi-audio tracks natively in MPV.
- **Smart Flow**: Anime movies skip directly to high-health streams; TV series provide an intuitive episode stepper with `n` / `P` continuous binge-watching.

### 3. 🎨 Cartoons & Animation (TVMaze • Classic Cartoons • Multi-Source Indexers)
Dedicated streaming mode for Western animation, Saturday morning classics, and modern animated hits:
- **Curated Evergreen Catalog**: Immediate instant access to timeless favorites without needing to type (*Adventure Time*, *Avatar: The Last Airbender*, *Gravity Falls*, *Rick and Morty*, *Batman: The Animated Series*, *SpongeBob SquarePants*, *The Simpsons*, *Tom and Jerry*, *Archer*, *DuckTales*, *Samurai Jack*, *South Park*, etc.).
- **TVMaze Animation Search**: Real-time queries with automatic genre filtering, network attribution (Cartoon Network, Nickelodeon, Adult Swim, Disney, Fox, Comedy Central), ratings, and air dates.
- **Interactive Season & Episode Stepper**: Smooth keyboard stepper allowing quick field switching between Season and Episode with direct numeric input or arrow keys.
- **Multi-Source Torrent Aggregation**: Aggregates releases across **Torrentio**, **The Pirate Bay (Apibay)**, **Torrents-CSV**, and **Nyaa.si** for full episode packs and individual episode streams.
- **Binge-Watching Shortcuts**: Jump straight into the next episode with `n` or previous with `P`.

### 4. 📺 Live TV Streaming (IPTV-org)
- Browse and stream **11,000+ live television channels** worldwide.
- Category filtering: **News**, **Movies**, **Sports**, **Entertainment**, **Music**, **Documentary**, **Kids**, **General**.
- Real-time search by channel name or country.
- Instant HLS (`.m3u8`) playback directly inside MPV with automatic stream recovery.

### 5. 🏛 Internet Archive (Archive.org)
- Search and stream thousands of **public-domain feature films**, cult cinema, vintage sci-fi, horror, historical documentaries, and silent classics (e.g. *Night of the Living Dead*, *Plan 9 from Outer Space*, *Metropolis*, *Charlie Chaplin*, *His Girl Friday*).
- Direct seekable HTTP video playback (`.mp4`, `.mkv`) with zero reliance on P2P seeders.

### 6. 🔗 Direct Magnet & Stream URL Player
- Paste any `magnet:?xt=urn:btih:...` link, 40-character hex infohash, or direct video URL (`.m3u8`, `.mp4`).
- Stream instantly with hardware-accelerated playback.

### 7. ⏱️ Continue Watching & Watch History (Auto-Resuming & Next Episode Binge)
- **Automatic History Persistence**: Automatically logs every played stream to local persistent storage (`~/.local/share/tsss-tui/history.json`).
- **One-Click Resume**: Replay any movie, anime, cartoon, TV series, or IPTV channel without searching again.
- **Instant Binge-Watching Advance**: Press `n` on any watched TV show, cartoon, or anime in the history list to automatically increment to the **Next Episode** (`S01E04` → `S01E05`) and immediately fetch stream sources.
- **Playback Timestamp Resuming**: Launches MPV with `--save-position-on-quit` to resume exact playback timestamps within video files.

---

## 🚀 Key Features

- **Blazingly Fast Parallel Fetching**: Background worker threads query Torrentio, Nyaa.si, Apibay, and Torrents-CSV simultaneously with zero UI freezing or dropped frames.
- **Smart Flow**: Movies skip directly to the best stream sources; TV and anime series provide intuitive Episode steppers.
- **Stream Inspector**: Dual-column layout displaying stream health badges, release title, exact file size, seeder/leecher breakdown, video codecs (`HEVC/x265`, `x264`, `AV1`), audio profiles, and HDR profiles (`Dolby Vision`, `HDR10+`).
- **Quick Filters & Sorting**:
  - Filter by Quality (`Tab` / `f`): `All`, `4K`, `1080p`, `720p`, `480p`
  - Filter by Provider (`p`): `All Providers`, `NyaaSi`, `Torrentio`, `ThePirateBay`, `Torrents-CSV`
  - Sort Order (`s`): `By Seeders` (Swarm health), `By Quality` (Resolution), `By Size`
- **Binge-Watching Support**: Press `n` anytime in the source picker to jump to the next episode, or `P` for the previous episode without re-searching.
- **Automatic Subtitles & Multi-Audio**: Integrated with OpenSubtitles v3 for live/movie subtitles, and full native support for styled MKV anime soft-subs (ASS/SSA) and Japanese/English dual-audio tracks.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Description |
|---|---|
| **F1** / **Ctrl+1** | Switch to **Movies & TV Series** mode |
| **F2** / **Ctrl+2** | Switch to **Anime Streaming** mode |
| **F3** / **Ctrl+3** | Switch to **Cartoons & Animation** mode |
| **F4** / **Ctrl+4** | Switch to **Live TV (IPTV)** mode |
| **F5** / **Ctrl+5** | Switch to **Internet Archive** mode |
| **F6** / **Ctrl+6** | Switch to **Direct Stream** mode |
| **F7** / **Ctrl+7** / **H** | Switch to **Continue Watching (Watch History)** mode |
| **?** | Toggle full **Keyboard Help Overlay** |
| **↑ / ↓**, **j / k** | Navigate list items |
| **1 – 9** | Quick-select item by number |
| **Tab** / **↑ / ↓** | Switch between Season & Episode in Cartoons/Series Stepper |
| **← / →**, **- / +** | Adjust Season or Episode in Stepper |
| **0 – 9** | Type Season or Episode directly |
| **Tab** / **f** | Cycle quality filter (`All` → `4K` → `1080p` → `720p` → `480p`) |
| **p** | Cycle provider filter (`All` → `NyaaSi` → `Torrentio` → `ThePirateBay` → `Torrents-CSV`) |
| **s** | Cycle sort order (`Seeders` → `Quality` → `Size`) |
| **n** / **N** | Binge-watching: Jump to **Next Episode** |
| **P** | Jump to **Previous Episode** |
| **d** | Remove entry from Continue Watching / History |
| **c** | Clear entire Watch History |
| **Enter** | Confirm title / episode / stream in MPV |
| **q** / **Esc** | Go back / clear search |
| **Ctrl+C** | Quit application |

---

## ⚙️ Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `SSS_TORRENTIO` | `https://torrentio.strem.fun` | Custom or self-hosted Torrentio instance URL |
| `SSS_SUBS` | `eng` | Subtitle language code (`eng`, `spa`, `fre`, `ger`, or `off`) |
| `SSS_PLAYER` | `mpv` | Media player (`mpv` or `ffplay`) |
| `SSS_PATH` | `/home/lk/webseries/sss.txt` | Path to `sss` streaming backend script |
| `SSS_TORRENT_WAIT` | `90` | Seconds to wait for initial BitTorrent peers before timeout |

---

## 🛠️ Build & Run

```bash
# Build release binary
cargo build --release

# Run interactive TUI
cargo run --release

# Run unit tests
cargo test
```
