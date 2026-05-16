use ratatui::widgets::ListState;
use rustypipe::{
    client::RustyPipe,
    model::{YouTubeItem, traits::YtEntity},
};
use std::{
    io::{self, ErrorKind},
    process::{Command, Output},
};
use tui_input::Input;

use crate::trace_dbg;

const YT_BASE_URL: &str = r"https://www.youtube.com/watch?v=";

#[derive(Debug, Clone)]
pub enum VideoPlayer {
    MPV,
    IINA,
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
impl YoutubeSearchError {
    pub fn to_string(self) -> String {
        match self {
            YoutubeSearchError::EmptySearch => return "Empty Search!".to_string(),
            YoutubeSearchError::NoResult => return "No Results Found!".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct YoutubeResult {
    pub video_title: String,
    pub url: String,
}
pub struct App {
    pub show_debug: bool,
    pub debug_text: String,
    pub state: AppState,
    pub user_search_input: Input,
    pub search_state: ListState,
    pub search_results: Vec<YoutubeResult>,
    client: RustyPipe,
    player: VideoPlayer,
}

impl App {
    pub fn new() -> Self {
        let player = find_player();

        Self {
            show_debug: true,
            debug_text: String::default(),
            state: AppState::Main,
            user_search_input: Input::new("".into()),
            search_state: ListState::default(),
            search_results: Vec::new(),
            client: RustyPipe::new(), // Change this to use the `builder` eventually
            player: player,
        }
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

        let results: rustypipe::model::SearchResult<YouTubeItem> = self
            .client
            .query()
            .search(&self.user_search_input.value())
            .await
            .unwrap();

        if results.items.items.is_empty() {
            self.debug_text = format!("No results found {results:?}");
            return Err(YoutubeSearchError::NoResult);
        }

        let results = results.items.items.iter();
        for result in results {
            if let YouTubeItem::Video(r) = result {
                trace_dbg!(r);
                self.search_results.push(YoutubeResult {
                    video_title: r.name().to_string(),
                    url: r.id().to_string(),
                });
            }
        }
        self.debug_text = format!("Results Found! {0:?}", self.search_results);
        Ok(())
    }

    pub async fn try_launch_video(&mut self) -> () {
        self.state = AppState::Loading;
        self.debug_text = "Searching for video".to_string();

        let selected_video = self.search_results[self.search_state.selected().unwrap()].clone();

        let url = YT_BASE_URL.to_string() + &selected_video.url;

        self.debug_text = "Launching video".to_string();

        let mut status: Result<Output, io::Error> = Err(io::Error::new(
            ErrorKind::NotFound,
            "Could not load video player",
        ));

        match self.player {
            VideoPlayer::MPV => {
                status = Command::new("mpv").arg(url).output();
            }
            VideoPlayer::IINA => {
                let args = vec!["-a", "IINA", &url];
                status = Command::new("open").args(args).output();
            }
        }

        if let Ok(_) = status {
            self.debug_text = "Done Watching".to_string();
            self.state = AppState::Main;
        } else if let Err(err) = status {
            self.debug_text = format!("SOMETHING WENT WRONG WITH MPV {err}");
        }
    }
}

fn find_player() -> VideoPlayer {
    if cfg!(target_os = "macos") {
        VideoPlayer::IINA
    } else {
        VideoPlayer::MPV
    }
}
