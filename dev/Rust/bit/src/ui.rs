use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, Mode};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const CURRENT_LINE_BG: Color = Color::Rgb(40, 44, 52);
const TITLE_BG: Color = Color::Rgb(20, 22, 28);

pub fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_title_bar(frame, chunks[0], app);
    draw_editor(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);

    match &app.mode {
        Mode::SaveAs { input } => draw_save_as_popup(frame, area, input),
        Mode::ConfirmQuit => draw_confirm_quit_popup(frame, area),
        Mode::Normal => {}
    }
}

fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let name = app
        .filename
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "[New Buffer]".to_string());
    let modified = if app.modified { " *" } else { "" };

    let line = Line::from(vec![
        Span::styled(
            " BIT ",
            Style::default()
                .bg(ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(name, Style::default().fg(Color::White)),
        Span::styled(modified, Style::default().fg(Color::Yellow)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(TITLE_BG)),
        area,
    );
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let gutter_width = app.lines.len().to_string().len().max(2) as u16 + 2;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gutter_width), Constraint::Min(0)])
        .split(area);
    let gutter_area = cols[0];
    let text_area = cols[1];

    let visible_rows = text_area.height as usize;
    let visible_cols = text_area.width as usize;

    if app.cursor_row < app.row_offset {
        app.row_offset = app.cursor_row;
    } else if visible_rows > 0 && app.cursor_row >= app.row_offset + visible_rows {
        app.row_offset = app.cursor_row - visible_rows + 1;
    }

    if app.cursor_col < app.col_offset {
        app.col_offset = app.cursor_col;
    } else if visible_cols > 0 && app.cursor_col >= app.col_offset + visible_cols {
        app.col_offset = app.cursor_col - visible_cols + 1;
    }

    let end_row = (app.row_offset + visible_rows).min(app.lines.len());

    let mut gutter_lines = Vec::new();
    let mut text_lines = Vec::new();

    for row in app.row_offset..end_row {
        let is_cursor_row = row == app.cursor_row;

        let num_style = if is_cursor_row {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        gutter_lines.push(Line::from(Span::styled(
            format!("{:>width$} ", row + 1, width = (gutter_width - 1) as usize),
            num_style,
        )));

        let visible_text: String = app.lines[row]
            .chars()
            .skip(app.col_offset)
            .take(visible_cols)
            .collect();
        let line_style = if is_cursor_row {
            Style::default().bg(CURRENT_LINE_BG)
        } else {
            Style::default()
        };
        text_lines.push(Line::from(Span::styled(visible_text, line_style)));
    }

    frame.render_widget(Paragraph::new(gutter_lines), gutter_area);
    frame.render_widget(Paragraph::new(text_lines), text_area);

    let cursor_x = text_area.x + (app.cursor_col - app.col_offset) as u16;
    let cursor_y = text_area.y + (app.cursor_row - app.row_offset) as u16;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let hint = app
        .message
        .clone()
        .unwrap_or_else(|| " Ctrl+S save   Ctrl+Q quit".to_string());
    let pos = format!(" Ln {}, Col {} ", app.cursor_row + 1, app.cursor_col + 1);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(pos.len() as u16)])
        .split(area);

    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Black).bg(Color::Gray)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(pos)
            .style(Style::default().fg(Color::Black).bg(ACCENT))
            .alignment(Alignment::Right),
        chunks[1],
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn draw_save_as_popup(frame: &mut Frame, area: Rect, input: &str) {
    let popup = centered_rect(50, 3, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Save as ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));
    frame.render_widget(Paragraph::new(format!("{input}_")).block(block), popup);
}

fn draw_confirm_quit_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(46, 3, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new("Save before exit? (y/n, esc to cancel)").block(block),
        popup,
    );
}
