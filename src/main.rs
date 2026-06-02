use app::{App, AppState};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    error::Error,
    fs::{self},
    io::{self, Read, Write},
    path::Path,
};
use tui_input::backend::crossterm::EventHandler;
use ui::draw_ui;
mod app;
mod ui;

#[derive(serde::Deserialize, serde::Serialize)]
struct Config {
    player: String,
}
impl Config {
    fn new() -> Self {
        Self {
            player: String::from("iina"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Read/setup config file if it exists.
    let home = std::env::home_dir().unwrap_or_default();
    let config_file_root = Path::join(home.as_path(), Path::new(".config/youtuber/"));
    let config_file_path = Path::join(&config_file_root, "config.json");

    fs::create_dir_all(&config_file_root)?;

    let config = match fs::OpenOptions::new().read(true).open(&config_file_path) {
        Ok(mut f) => {
            let mut buf = String::new();
            f.read_to_string(&mut buf).unwrap();
            let c: Config = serde_json::from_str(buf.as_str()).unwrap();
            c
        }
        Err(_) => {
            // file doesn't exist, so we can write to it
            let c = Config::new();
            let buf = serde_json::to_string(&c).unwrap();
            _ = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&config_file_path)
                .unwrap()
                .write(buf.as_bytes());
            c
        }
    };
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let response = match App::with_config(config) {
        Ok(mut app) => run_app(&mut terminal, &mut app).await,
        Err(_) => Err(std::io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to find compatible player. Ensure 'mpv' or 'iina' are installed",
        )),
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = response {
        println!("{err:?}")
    }
    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool>
where
    io::Error: From<B::Error>,
{
    loop {
        // Draw the app based on current state
        terminal.draw(|frame| draw_ui(frame, app))?;

        let event = event::read()?;
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match &app.state {
                app::AppState::Main => match key.code {
                    KeyCode::Char('i') => app.state = AppState::Searching,
                    KeyCode::Char('q') => app.state = AppState::Exiting,
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.search_state.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.search_state.select_previous();
                    }
                    KeyCode::Enter => app.try_launch_video().await,
                    _ => {}
                },
                // Return from the application with okay of error values
                app::AppState::Exiting => match key.code {
                    KeyCode::Char('n') | KeyCode::Esc => app.state = AppState::Main,
                    KeyCode::Char('y') | KeyCode::Char('q') => return Ok(true),
                    _ => {}
                },
                app::AppState::Searching => match key.code {
                    KeyCode::Esc => app.state = AppState::Main,
                    KeyCode::Enter => app.execute_search().await,
                    _ => {
                        app.user_search_input.handle_event(&event);
                    }
                },
                app::AppState::Loading => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::Main,
                    _ => {}
                },
                app::AppState::Error(err) => match err {
                    app::YoutubeSearchError::EmptySearch => {
                        app.state = AppState::Error(err.clone())
                    }
                    app::YoutubeSearchError::NoResult => app.state = AppState::Error(err.clone()),
                },
            }
        }
    }
}
