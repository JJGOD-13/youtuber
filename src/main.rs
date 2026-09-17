use app::{App, AppState};
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::{Backend, CrosstermBackend},
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

#[derive(serde::Deserialize, serde::Serialize, Default)]
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

    let config = fs::OpenOptions::new()
        .read(true)
        .open(&config_file_path)
        .map_or_else(
            |_| {
                // File doesn't exist, so we can write to it
                let c = Config::new();
                let buf = serde_json::to_string(&c).unwrap_or_default();
                if let Ok(mut f) = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&config_file_path)
                {
                    let _ = f.write_all(buf.as_bytes());
                }
                c
            },
            |mut f| {
                let mut buf = String::new();
                f.read_to_string(&mut buf).unwrap_or_default();
                let c: Config = serde_json::from_str(buf.as_str()).unwrap_or_default();
                c
            },
        );
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    if let Ok(mut app) = App::with_config(&config) {
        let _ = run_app(&mut terminal, &mut app).await;
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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
                    KeyCode::Enter => app.launch_video(),
                    _ => {}
                },
                // Return from the application with okay of error values
                app::AppState::Exiting => match key.code {
                    KeyCode::Char('n') | KeyCode::Esc => app.state = AppState::Main,
                    KeyCode::Char('y' | 'q') => return Ok(true),
                    _ => {}
                },
                app::AppState::Searching => match key.code {
                    KeyCode::Esc => app.state = AppState::Main,
                    KeyCode::Char('q') => app.state = AppState::Exiting,
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
                        app.state = AppState::Error(err.clone());
                    }
                    app::YoutubeSearchError::NoResult => app.state = AppState::Error(err.clone()),
                },
            }
        }
    }
}
