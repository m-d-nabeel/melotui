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
    // 1. Calculate Centered Rect
    // We use a vertical layout first to squish the height, then horizontal for width.
    let popup_layout_v = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(25), // Fixed height for consistency
        Constraint::Fill(1),
    ])
    .split(area);

    let popup_area = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(60), // Fixed width
        Constraint::Fill(1),
    ])
    .split(popup_layout_v[1])[1];

    // --- STYLES ---
    let border_style = Style::default().fg(Color::Cyan);
    let title_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Header Row Style: Dark background to create a "bar" effect
    let category_row_style = Style::default().bg(Color::Rgb(40, 40, 40));
    let category_text_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let key_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);

    // 2. Prepare Table Data
    let docs = InputHandler::get_help_docs();
    let mut rows = Vec::new();
    let mut last_category = "";

    for item in docs {
        // If category changes, insert a "Section Header" row
        if item.category != last_category {
            rows.push(
                Row::new(vec![
                    Cell::from(format!("  {}  ", item.category.to_uppercase()))
                        .style(category_text_style),
                    Cell::from(""), // Empty second cell, but background covers it
                ])
                .style(category_row_style) // Apply background to whole row
                .height(1),
            );
            last_category = item.category;
        }

        // Key Binding Row
        rows.push(Row::new(vec![
            Cell::from(format!("  {}", item.key)).style(key_style),
            Cell::from(item.desc).style(desc_style),
        ]));
    }

    // 3. Build the Table Widget
    let table = Table::new(
        rows,
        [
            Constraint::Length(15), // Fixed width for Keys (keeps alignment clean)
            Constraint::Fill(1),    // Description takes remaining space
        ],
    )
    .block(
        Block::default()
            .title(
                Line::from(" Keyboard Shortcuts ")
                    .alignment(Alignment::Center)
                    .style(title_style),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Thick) // Thick border looks cleaner
            .border_style(border_style)
            // No internal padding on the block itself to allow header rows to touch edges if desired,
            // but we add a bit to keep text off the border.
            .padding(Padding::horizontal(1)),
    );

    // 4. Render
    frame.render_widget(Clear, popup_area); // Clear background
    frame.render_widget(table, popup_area); // Draw popup
}
