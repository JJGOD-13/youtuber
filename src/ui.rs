use crate::app::{App, AppState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{self, Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, List, Padding, Paragraph},
};
use std::rc::Rc;

pub fn draw_ui(frame: &mut Frame, app: &mut App) {
    let chunks: Rc<[Rect]> = define_ui_outline(frame);

    render_searchbar(frame, app, &chunks);
    render_search_results(frame, app, &chunks);
    render_exit_popup(frame, app);
    render_bottom_bar(frame, app, &chunks);
}

fn render_search_results(frame: &mut Frame<'_>, app: &mut App, chunks: &[Rect]) {
    let search_results_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default())
        .title("Results");

    let list = List::new(app.search_results.iter().map(|i| i.video_title.clone()))
        .style(Color::White)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol(">")
        .block(search_results_block);

    frame.render_stateful_widget(list, chunks[1], &mut app.search_state);
}

fn render_bottom_bar(frame: &mut Frame<'_>, app: &App, chunks: &Rc<[Rect]>) {
    let footer_chunks = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(88), Constraint::Percentage(12)],
    )
    .split(chunks[2]);

    if app.show_debug {
        let debug_text = Text::from(app.debug_text.clone()).style(Style::new().fg(Color::White));
        frame.render_widget(debug_text, footer_chunks[0]);
    } else {
        let help_text = Text::from("[i -> Search for video] [j/k -> Navigate results] [enter -> Select video] [q -> quit] ").style(Style::new().fg(Color::DarkGray));
        frame.render_widget(help_text, footer_chunks[0]);
    }

    let bottom_bar = Block::default().padding(Padding {
        left: 0,
        right: 1,
        top: 0,
        bottom: 0,
    });

    let state_string = match &app.state {
        AppState::Main => Line::from("Normal").style(Style::new().fg(Color::White)),
        AppState::Searching => Line::from("Searching").style(Style::new().fg(Color::Yellow)),
        AppState::Loading => Line::from("Loading...").style(Style::new().fg(Color::White)),
        AppState::Exiting => Line::from("Exiting").style(Style::new().fg(Color::Red)),
        AppState::Error(youtube_search_error) => {
            Line::from(youtube_search_error.clone().to_string()).style(Style::new().fg(Color::Red))
        }
    };

    let state_text = Paragraph::new(state_string)
        .style(Style::new().bold())
        .right_aligned()
        .block(bottom_bar);

    frame.render_widget(state_text, footer_chunks[1]);
}

fn render_searchbar(frame: &mut Frame<'_>, app: &App, chunks: &Rc<[Rect]>) {
    let search_bar = Block::default()
        .borders(Borders::ALL)
        .style(Style::default())
        .title("Youtuber");

    let search_input = Paragraph::new(app.user_search_input.value())
        .style(style::Color::White)
        .block(search_bar);

    // Set cursor blinking and position:
    match app.state {
        AppState::Searching => {
            let x = app.user_search_input.visual_cursor() + 1;
            frame.set_cursor_position((chunks[0].x + x as u16, chunks[0].y + 1));
        }
        _ => {}
    }

    frame.render_widget(search_input, chunks[0]);
}

fn render_exit_popup(frame: &mut Frame<'_>, app: &App) {
    if let AppState::Exiting = app.state {
        frame.render_widget(Clear, frame.area());

        let raw_text = Text::raw("Would you like to exit? (Y/N)");
        let exit_text = Paragraph::new(raw_text.clone())
            .block(Block::bordered())
            .centered();

        let centered_rect = frame
            .area()
            .centered(Constraint::Percentage(20), Constraint::Length(3));

        frame.render_widget(exit_text, centered_rect);
    }
}

fn define_ui_outline(frame: &Frame) -> Rc<[Rect]> {
    Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage(5),
            Constraint::Fill(1),
            Constraint::Percentage(5),
        ])
        .split(frame.area())
}
