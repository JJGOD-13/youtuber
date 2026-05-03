use app::{App, AppState};
use logging::initialize_logging;
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::{Backend, CrosstermBackend},
    Terminal,
};
use std::{error::Error, io};
use tui_input::backend::crossterm::EventHandler;
use ui::draw_ui;
mod app;
mod logging;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    initialize_logging()?;
    _ = enable_raw_mode();
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    trace_dbg!("Starting app");
    let mut app = App::new();
    let response = run_app(&mut terminal, &mut app).await;

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
                    KeyCode::Char('j') => app.search_state.select_next(),
                    KeyCode::Char('k') => app.search_state.select_previous(),
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
