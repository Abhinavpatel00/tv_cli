use crate::models::{CartoonItem, SourceItem, StreamSource};
use crate::sources::anime::fetch_nyaa_anime;
use crate::sources::piratebay;
use crate::sources::search_title;
use crate::sources::torrentio;
use crate::sources::torrents_csv;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TVMAZE_SEARCH: &str = "https://api.tvmaze.com/search/shows?q={}";

#[derive(Deserialize, Debug)]
struct TvMazeRating {
    average: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct TvMazeNetwork {
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TvMazeExternals {
    imdb: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TvMazeShow {
    id: u64,
    name: String,
    #[serde(rename = "type")]
    show_type: Option<String>,
    genres: Option<Vec<String>>,
    status: Option<String>,
    premiered: Option<String>,
    rating: Option<TvMazeRating>,
    network: Option<TvMazeNetwork>,
    #[serde(rename = "webChannel")]
    web_channel: Option<TvMazeNetwork>,
    externals: Option<TvMazeExternals>,
    summary: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TvMazeSearchResult {
    show: TvMazeShow,
}

pub fn clean_html_summary(html: &str) -> String {
    let mut in_tag = false;
    let mut result = String::new();
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&rsquo;", "'")
        .replace("&lsquo;", "'")
        .replace("&rdquo;", "\"")
        .replace("&ldquo;", "\"")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

pub fn get_curated_popular_cartoons() -> Vec<CartoonItem> {
    vec![
        CartoonItem {
            id: "290".to_string(),
            imdb_id: Some("tt1305826".to_string()),
            title: "Adventure Time".to_string(),
            year: "2010".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Adventure".to_string(), "Comedy".to_string(), "Fantasy".to_string()],
            network: Some("Cartoon Network".to_string()),
            status: "Ended".to_string(),
            summary: "A 12-year-old boy and his wise 28-year-old dog with magical powers go on a series of surreal adventures with princesses, vampires, and monsters in the post-apocalyptic Land of Ooo.".to_string(),
            rating: Some("8.6".to_string()),
        },
        CartoonItem {
            id: "216".to_string(),
            imdb_id: Some("tt2861424".to_string()),
            title: "Rick and Morty".to_string(),
            year: "2013".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Sci-Fi".to_string(), "Comedy".to_string()],
            network: Some("Adult Swim".to_string()),
            status: "Running".to_string(),
            summary: "An animated series that follows the exploits of a super scientist and his not-so-bright grandson across interdimensional space and alternate realities.".to_string(),
            rating: Some("9.1".to_string()),
        },
        CartoonItem {
            id: "555".to_string(),
            imdb_id: Some("tt0417299".to_string()),
            title: "Avatar: The Last Airbender".to_string(),
            year: "2005".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Adventure".to_string(), "Fantasy".to_string()],
            network: Some("Nickelodeon".to_string()),
            status: "Ended".to_string(),
            summary: "In a war-torn world of elemental magic, a young boy reawakens to undertake a dangerous mystic quest to fulfill his destiny as the Avatar and bring peace to the world.".to_string(),
            rating: Some("9.3".to_string()),
        },
        CartoonItem {
            id: "878".to_string(),
            imdb_id: Some("tt1865718".to_string()),
            title: "Gravity Falls".to_string(),
            year: "2012".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Mystery".to_string(), "Comedy".to_string(), "Adventure".to_string()],
            network: Some("Disney Channel".to_string()),
            status: "Ended".to_string(),
            summary: "Twin siblings Dipper and Mabel Pines spend the summer at their great-uncle's tourist trap in the enigmatic town of Gravity Falls, Oregon, unlocking cryptic supernatural secrets.".to_string(),
            rating: Some("8.9".to_string()),
        },
        CartoonItem {
            id: "44458".to_string(),
            imdb_id: Some("tt11126994".to_string()),
            title: "Arcane".to_string(),
            year: "2021".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Sci-Fi".to_string(), "Drama".to_string()],
            network: Some("Netflix".to_string()),
            status: "Running".to_string(),
            summary: "Set in the utopian region of Piltover and the oppressed underground of Zaun, the story follows the origins of two iconic League champions-and the power that will tear them apart.".to_string(),
            rating: Some("9.0".to_string()),
        },
        CartoonItem {
            id: "112".to_string(),
            imdb_id: Some("tt0121955".to_string()),
            title: "South Park".to_string(),
            year: "1997".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Comedy".to_string()],
            network: Some("Comedy Central".to_string()),
            status: "Running".to_string(),
            summary: "Follows the misadventures of four irreverent grade-schoolers in the quiet, dysfunctional mountain town of South Park, Colorado.".to_string(),
            rating: Some("8.7".to_string()),
        },
        CartoonItem {
            id: "526".to_string(),
            imdb_id: Some("tt0206512".to_string()),
            title: "SpongeBob SquarePants".to_string(),
            year: "1999".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Comedy".to_string(), "Family".to_string()],
            network: Some("Nickelodeon".to_string()),
            status: "Running".to_string(),
            summary: "The character-driven cartoon chronicles the nautical and often nonsensical adventures of SpongeBob, an incurably optimistic and earnest sea sponge, and his undersea friends.".to_string(),
            rating: Some("8.2".to_string()),
        },
        CartoonItem {
            id: "442".to_string(),
            imdb_id: Some("tt1710308".to_string()),
            title: "Regular Show".to_string(),
            year: "2010".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Comedy".to_string(), "Adventure".to_string()],
            network: Some("Cartoon Network".to_string()),
            status: "Ended".to_string(),
            summary: "The daily surreal adventures of a blue jay and a raccoon who work as groundskeepers at a local park and spend their time slacking off.".to_string(),
            rating: Some("8.5".to_string()),
        },
        CartoonItem {
            id: "712".to_string(),
            imdb_id: Some("tt0103359".to_string()),
            title: "Batman: The Animated Series".to_string(),
            year: "1992".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Crime".to_string(), "Drama".to_string()],
            network: Some("Fox Kids".to_string()),
            status: "Ended".to_string(),
            summary: "The Dark Knight battles crime and corruption in Gotham City alongside Robin and Batgirl in this groundbreaking, noir-inspired animated classic.".to_string(),
            rating: Some("9.0".to_string()),
        },
        CartoonItem {
            id: "1665".to_string(),
            imdb_id: Some("tt0279077".to_string()),
            title: "Samurai Jack".to_string(),
            year: "2001".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Adventure".to_string(), "Sci-Fi".to_string()],
            network: Some("Cartoon Network".to_string()),
            status: "Ended".to_string(),
            summary: "A samurai warrior sent through time to a dystopian future ruled by the shape-shifting demon Aku must seek a way back to the past to undo the evil that is Aku.".to_string(),
            rating: Some("8.5".to_string()),
        },
        CartoonItem {
            id: "172".to_string(),
            imdb_id: Some("tt0096697".to_string()),
            title: "The Simpsons".to_string(),
            year: "1989".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Comedy".to_string()],
            network: Some("FOX".to_string()),
            status: "Running".to_string(),
            summary: "The satiric adventures of a working-class family in the misfit city of Springfield.".to_string(),
            rating: Some("8.7".to_string()),
        },
        CartoonItem {
            id: "84".to_string(),
            imdb_id: Some("tt0149460".to_string()),
            title: "Futurama".to_string(),
            year: "1999".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Sci-Fi".to_string(), "Comedy".to_string()],
            network: Some("Hulu".to_string()),
            status: "Running".to_string(),
            summary: "Philip J. Fry, a pizza delivery boy, is accidentally frozen in 1999 and thawed out on New Year's Eve 2999 into a wild futuristic universe.".to_string(),
            rating: Some("8.5".to_string()),
        },
        CartoonItem {
            id: "688".to_string(),
            imdb_id: Some("tt0374463".to_string()),
            title: "Star Wars: The Clone Wars".to_string(),
            year: "2008".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Sci-Fi".to_string()],
            network: Some("Cartoon Network".to_string()),
            status: "Ended".to_string(),
            summary: "Jedi Knights lead the Grand Army of the Republic against the droid armies of the Separatists across the galaxy.".to_string(),
            rating: Some("8.4".to_string()),
        },
        CartoonItem {
            id: "36706".to_string(),
            imdb_id: Some("tt6741278".to_string()),
            title: "Invincible".to_string(),
            year: "2021".to_string(),
            is_series: true,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Drama".to_string()],
            network: Some("Amazon Prime".to_string()),
            status: "Running".to_string(),
            summary: "An adult animated superhero series that revolves around 17-year-old Mark Grayson, who's just like every other guy his age - except his father is the most powerful superhero on the planet.".to_string(),
            rating: Some("8.7".to_string()),
        },
        CartoonItem {
            id: "movie_spiderverse".to_string(),
            imdb_id: Some("tt4633694".to_string()),
            title: "Spider-Man: Into the Spider-Verse".to_string(),
            year: "2018".to_string(),
            is_series: false,
            genres: vec!["Animation".to_string(), "Action".to_string(), "Adventure".to_string(), "Sci-Fi".to_string()],
            network: Some("Sony Pictures Animation".to_string()),
            status: "Released".to_string(),
            summary: "Teen Miles Morales becomes the new Spider-Man and joins other Spider-Heroes from parallel dimensions to stop a threat to all reality.".to_string(),
            rating: Some("8.4".to_string()),
        },
        CartoonItem {
            id: "movie_lionking".to_string(),
            imdb_id: Some("tt0110357".to_string()),
            title: "The Lion King".to_string(),
            year: "1994".to_string(),
            is_series: false,
            genres: vec!["Animation".to_string(), "Adventure".to_string(), "Drama".to_string()],
            network: Some("Walt Disney Feature Animation".to_string()),
            status: "Released".to_string(),
            summary: "Lion prince Simba and his father are targeted by his bitter uncle, who wants to ascend the throne himself in this universally acclaimed animated masterpiece.".to_string(),
            rating: Some("8.5".to_string()),
        },
    ]
}

pub fn search_cartoons(query: &str) -> Result<Vec<CartoonItem>, Box<dyn std::error::Error>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(get_curated_popular_cartoons());
    }

    let encoded = urlencoding::encode(q);
    let url = TVMAZE_SEARCH.replace("{}", &encoded);

    let text = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(7))
        .call()?
        .into_string()?;

    let results: Vec<TvMazeSearchResult> = serde_json::from_str(&text).unwrap_or_default();
    let mut out = Vec::new();

    for r in results {
        let s = r.show;
        let is_anim = s.show_type.as_deref() == Some("Animation")
            || s.genres.as_ref().is_some_and(|g| g.iter().any(|x| x.eq_ignore_ascii_case("Animation")));

        let year = s
            .premiered
            .as_deref()
            .map(|d| if d.len() >= 4 { d[..4].to_string() } else { d.to_string() })
            .unwrap_or_else(|| "----".to_string());

        let network = s
            .network
            .and_then(|n| n.name)
            .or_else(|| s.web_channel.and_then(|w| w.name));

        let summary = s
            .summary
            .map(|html| clean_html_summary(&html))
            .unwrap_or_else(|| "No synopsis available.".to_string());

        let rating = s.rating.and_then(|r| r.average).map(|v| format!("{v:.1}"));

        let imdb_id = s.externals.and_then(|e| e.imdb);

        let mut genres = s.genres.unwrap_or_default();
        if is_anim && !genres.iter().any(|g| g.eq_ignore_ascii_case("Animation")) {
            genres.insert(0, "Animation".to_string());
        }

        out.push(CartoonItem {
            id: s.id.to_string(),
            imdb_id,
            title: s.name,
            year,
            is_series: true,
            genres,
            network,
            status: s.status.unwrap_or_else(|| "Unknown".to_string()),
            summary,
            rating,
        });

        // Limit results to top 15
        if out.len() >= 15 {
            break;
        }
    }

    // If TVMaze didn't return any animation, fallback to IMDb suggestion search
    if out.is_empty() {
        if let Ok(imdb_results) = search_title(q) {
            for ir in imdb_results {
                let yr = ir.year_str();
                let is_series = ir.is_series();
                let title = ir.l.unwrap_or_else(|| "Animated Title".to_string());
                out.push(CartoonItem {
                    id: ir.id.clone(),
                    imdb_id: Some(ir.id),
                    title,
                    year: yr,
                    is_series,
                    genres: vec!["Animation".to_string()],
                    network: Some("Feature Film".to_string()),
                    status: "Released".to_string(),
                    summary: format!("Starring: {}", ir.s.unwrap_or_else(|| "Animation Cast".to_string())),
                    rating: None,
                });
                if out.len() >= 10 {
                    break;
                }
            }
        }
    }

    Ok(out)
}

pub fn fetch_all_cartoon_sources(
    cartoon: &CartoonItem,
    season: u32,
    episode: u32,
) -> Vec<SourceItem> {
    let (tx, rx) = mpsc::channel::<Vec<SourceItem>>();

    // Worker 1: Torrentio via IMDb ID (if available)
    if let Some(imdb_id) = cartoon.imdb_id.clone() {
        let tx = tx.clone();
        let is_series = cartoon.is_series;
        thread::spawn(move || {
            let res = torrentio::fetch_torrentio(&imdb_id, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 2: The Pirate Bay (Apibay)
    {
        let tx = tx.clone();
        let title = cartoon.title.clone();
        let yr = cartoon.year.clone();
        let is_series = cartoon.is_series;
        thread::spawn(move || {
            let res = piratebay::fetch_piratebay(&title, &yr, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 3: Torrents-CSV
    {
        let tx = tx.clone();
        let title = cartoon.title.clone();
        let yr = cartoon.year.clone();
        let is_series = cartoon.is_series;
        thread::spawn(move || {
            let res = torrents_csv::fetch_torrents_csv(&title, &yr, is_series, season, episode);
            let _ = tx.send(res);
        });
    }

    // Worker 4: Nyaa.si (for animated shows with fansub releases)
    {
        let tx = tx.clone();
        let title = cartoon.title.clone();
        let is_movie = !cartoon.is_series;
        thread::spawn(move || {
            let res = fetch_nyaa_anime(&title, is_movie, episode);
            let _ = tx.send(res);
        });
    }

    drop(tx);

    let mut combined: Vec<SourceItem> = Vec::new();
    while let Ok(items) = rx.recv() {
        combined.extend(items);
    }

    // Deduplicate and merge by info_hash
    let mut deduplicated: Vec<SourceItem> = Vec::new();
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for item in combined {
        let hash_opt = match &item.stream {
            StreamSource::Torrent { info_hash, .. } => Some(info_hash.to_lowercase()),
            _ => None,
        };

        if let Some(hash) = hash_opt {
            if let Some(&existing_idx) = seen_hashes.get(&hash) {
                let existing = &mut deduplicated[existing_idx];

                if !existing.provider.contains(&item.provider) {
                    existing.provider = format!("{} + {}", existing.provider, item.provider);
                }

                if let Some(s) = item.seeds {
                    existing.seeds = Some(existing.seeds.unwrap_or(0).max(s));
                }

                if existing.size_bytes.is_none() && item.size_bytes.is_some() {
                    existing.size_bytes = item.size_bytes;
                    existing.size_str = item.size_str;
                }
                continue;
            }
            seen_hashes.insert(hash, deduplicated.len());
        }

        deduplicated.push(item);
    }

    // Default sort by seeders (healthiest swarm first)
    deduplicated.sort_by(|a, b| {
        let seeds_b = b.seeds.unwrap_or(0);
        let seeds_a = a.seeds.unwrap_or(0);
        seeds_b.cmp(&seeds_a)
    });

    // Re-number 1..N
    for (idx, item) in deduplicated.iter_mut().enumerate() {
        item.num = (idx + 1).to_string();
    }

    deduplicated
}
