use crate::input::InputHandler;
use melotui_core::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Cell, Clear, Padding, Row, Table},
};

pub fn draw(frame: &mut Frame, area: Rect, _state: &AppState) {
    // Center the popup
    let popup_layout_v = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(25), // Fixed height
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_area = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(60), // Fixed width
        Constraint::Fill(1),
    ])
    .split(popup_layout_v[1])[1];

    // Styles
    let border_style = Style::default().fg(Color::Cyan);
    let title_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let key_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);

    // Prepare table data
    let docs = InputHandler::get_help_docs();
    let mut rows = Vec::new();

    for item in docs {
        // Format the key nicely (e.g. "Char('q')" -> "q")
        let key_str = format!("{:?}", item.key)
            .replace("Char('", "")
            .replace("')", "");

        // Add row for binding
        rows.push(Row::new(vec![
            Cell::from(format!("  {}", key_str)).style(key_style),
            Cell::from(item.desc).style(desc_style),
        ]));
    }

    // Build table widget
    let table = Table::new(rows, [Constraint::Length(15), Constraint::Fill(1)]).block(
        Block::default()
            .title(
                Line::from(" Keyboard Shortcuts ")
                    .alignment(Alignment::Center)
                    .style(title_style),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(border_style)
            .padding(Padding::horizontal(1)),
    );

    // Render popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(table, popup_area);
}
