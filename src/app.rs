use bytes::Bytes;
use image::load_from_memory;
use ratatui::{layout::Size, widgets::ListState};
use ratatui_image::{picker::Picker, protocol::Protocol, FilterType::Nearest, Resize};
use rustypipe::{
    client::RustyPipe,
    model::{traits::YtEntity, YouTubeItem},
};
use std::{fmt::Display, process::Command};
use tui_input::Input;

use crate::Config;

const YT_BASE_URL: &str = r"https://www.youtube.com/watch?v=";

#[derive(Debug, Clone)]
pub enum VideoPlayer {
    Mpv,
    Iina,
}
impl Display for VideoPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoPlayer::Mpv => write!(f, "mpv"),
            VideoPlayer::Iina => write!(f, "iina"),
        }
    }
}

pub enum AppState {
    Main,
    Error(YoutubeSearchError),
    Exiting,
    Searching,
    Loading,
}
#[derive(Debug, Clone)]
pub enum YoutubeSearchError {
    EmptySearch,
    NoResult,
}
impl Display for YoutubeSearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YoutubeSearchError::EmptySearch => write!(f, "Empty Search!"),
            YoutubeSearchError::NoResult => write!(f, "No Results Found!"),
        }
    }
}

#[derive(Clone)]
pub struct YoutubeResult {
    pub video_title: String,
    pub url: String,
    pub thumbnail: Option<Protocol>,
}

impl std::fmt::Debug for YoutubeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YoutubeResult")
            .field("video_title", &self.video_title)
            .field("url", &self.url)
            .field("thumbnail", &self.url)
            .finish()
    }
}
pub struct App {
    pub show_debug: bool,
    pub debug_text: String,
    pub state: AppState,
    pub user_search_input: Input,
    pub search_state: ListState,
    pub search_results: Vec<YoutubeResult>,
    client: RustyPipe,
    pub player: VideoPlayer,
}

impl App {
    pub fn with_config(config: Config) -> Result<Self, which::Error> {
        let player = match config.player.to_lowercase().as_str() {
            "iina" => VideoPlayer::Iina,
            "mpv" => VideoPlayer::Mpv,
            _ => find_player(),
        };

        let show_debug = is_debug();

        Ok(Self {
            show_debug,
            debug_text: String::default(),
            state: AppState::Main,
            user_search_input: Input::new("".into()),
            search_state: ListState::default(),
            search_results: Vec::new(),
            client: RustyPipe::new(), // Change this to use the `builder` eventually
            player,
        })
    }

    pub fn clear_search_results(&mut self) {
        self.search_results = Vec::new();
        self.search_state = ListState::default();
    }

    pub async fn execute_search(&mut self) -> () {
        self.clear_search_results();
        self.state = AppState::Searching;
        if let Err(err) = self.search_youtube().await {
            self.state = AppState::Error(err);
        }
        self.state = AppState::Main;
    }

    pub async fn search_youtube(&mut self) -> Result<(), YoutubeSearchError> {
        if self.user_search_input.value().is_empty() {
            return Err(YoutubeSearchError::EmptySearch);
        }

        let results: rustypipe::model::SearchResult<YouTubeItem> = match self
            .client
            .query()
            .search(&self.user_search_input.value())
            .await
        {
            Ok(a) => a,
            Err(_) => return Err(YoutubeSearchError::NoResult),
        };

        if results.items.items.is_empty() {
            self.debug_text = format!("No results found {results:?}");
            return Err(YoutubeSearchError::NoResult);
        }

        let picker = Picker::from_query_stdio().unwrap();
        let font_size = picker.font_size();
        let results = results.items.items.iter();
        for result in results {
            if let YouTubeItem::Video(r) = result {
                // Get the thumbnail bytes too.

                let thumbnail_data = get_thumbnail_data(&picker, font_size, r).await;

                self.search_results.push(YoutubeResult {
                    video_title: r.name().to_string(),
                    url: r.id().to_string(),
                    thumbnail: thumbnail_data,
                });
            }
        }
        self.debug_text = format!("Results Found! {0:?}", self.search_results);
        self.search_state.select_first();
        Ok(())
    }

    pub async fn try_launch_video(&mut self) -> () {
        self.state = AppState::Loading;
        self.debug_text = "Searching for video".to_string();

        let idx = self.search_state.selected();
        if idx.is_none() {
            self.debug_text = "Haven't selected a video".to_string();
            self.state = AppState::Main;
            return;
        }
        let selected_video = self.search_results[self.search_state.selected().unwrap()].clone();

        let url = YT_BASE_URL.to_string() + &selected_video.url;

        self.debug_text = "Launching video".to_string();

        let status = match self.player {
            VideoPlayer::Mpv => Command::new("mpv").arg(url).output(),

            VideoPlayer::Iina => {
                let args = vec!["-a", "IINA", &url];
                Command::new("open").args(args).output()
            }
        };

        if status.is_ok() {
            self.debug_text = "Done Watching".to_string();
            self.state = AppState::Main;
        } else if let Err(err) = status {
            self.debug_text = format!("SOMETHING WENT WRONG WITH MPV {err}");
        }
    }
}

async fn get_thumbnail_data(
    picker: &Picker,
    font_size: ratatui_image::FontSize,
    r: &rustypipe::model::VideoItem,
) -> Option<Protocol> {
    if r.thumbnail.is_empty() {
        return None;
    }

    let bytes = get_bytes_from_url(r.thumbnail[0].url.clone()).await;
    let dyn_img = load_from_memory(&bytes).ok().unwrap();
    let size = Size::new(
        dyn_img.width().div_ceil(font_size.height as u32) as u16 * 4,
        dyn_img.height().div_ceil(font_size.height as u32) as u16 * 4,
    );

    Some(
        picker
            .new_protocol(dyn_img, size, Resize::Fit(Some(Nearest)))
            .unwrap(),
    )
}

async fn get_bytes_from_url(clone: String) -> Bytes {
    reqwest::get(clone).await.unwrap().bytes().await.unwrap()
}

#[cfg(debug_assertions)]
fn is_debug() -> bool {
    true
}

#[cfg(not(debug_assertions))]
fn is_debug() -> bool {
    false
}

fn find_player() -> VideoPlayer {
    if cfg!(target_os = "macos") {
        VideoPlayer::Iina
    } else {
        VideoPlayer::Mpv
    }
}
