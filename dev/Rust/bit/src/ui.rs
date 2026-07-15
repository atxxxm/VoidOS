use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, Buffer, Mode};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const CURRENT_LINE_BG: Color = Color::Rgb(40, 44, 52);
const TITLE_BG: Color = Color::Rgb(20, 22, 28);
const SELECTION_BG: Color = Color::Rgb(60, 90, 130);

// Plain ASCII border, not Unicode box-drawing: over a serial console or a
// terminal whose codepage isn't UTF-8, ratatui's default line-drawing
// characters (─│┌┐└┘) come out as mojibake. Plain "+-|" survives any
// encoding.
const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

fn bordered(style: Style) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
        .border_style(style)
}
const MATCH_BG: Color = Color::Yellow;

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

    draw_tab_bar(frame, chunks[0], app);
    draw_editor(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);

    match &app.mode {
        Mode::SaveAs { input } => draw_prompt_popup(frame, area, " Save as ", input),
        Mode::Open { input } => draw_prompt_popup(frame, area, " Open file ", input),
        Mode::Search { query, .. } => draw_search_popup(frame, area, query),
        Mode::Replace {
            find,
            replace,
            editing_replace,
        } => draw_replace_popup(frame, area, find, replace, *editing_replace),
        Mode::ConfirmQuit => draw_confirm_popup(
            frame,
            area,
            " Unsaved changes ",
            "Save all and quit? (y/n, esc to cancel)",
        ),
        Mode::ConfirmCloseTab => draw_confirm_popup(
            frame,
            area,
            " Unsaved changes ",
            "Save and close this tab? (y/n, esc to cancel)",
        ),
        Mode::Normal => {}
    }
}

fn draw_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(
            " BIT ",
            Style::default()
                .bg(ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    for (i, buf) in app.buffers.iter().enumerate() {
        let modified = if buf.modified { "*" } else { "" };
        let label = format!(" {}{} ", buf.display_name(), modified);
        let style = if i == app.active {
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(TITLE_BG)),
        area,
    );
}

// Per-row overlay info for build_line_spans, grouped into one struct so the
// function doesn't need a long, easy-to-misorder parameter list.
struct RowHighlight<'a> {
    selection: Option<(usize, usize)>,
    match_cols: &'a [usize],
    query_len: usize,
    char_styles: Option<&'a [Style]>,
}

// Splits a (already horizontally-scrolled) line into styled spans. Per-
// character base style comes from syntax highlighting (if available);
// selection and search-match highlighting are overlaid as backgrounds on
// top of it, then the current-line background tint if neither applies.
fn build_line_spans(
    line: &str,
    col_offset: usize,
    visible_cols: usize,
    is_cursor_row: bool,
    hl: &RowHighlight,
) -> Line<'static> {
    let chars: Vec<char> = line.chars().skip(col_offset).take(visible_cols).collect();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_style: Option<Style> = None;

    for (i, ch) in chars.iter().enumerate() {
        let abs_col = col_offset + i;
        let in_selection = hl.selection.is_some_and(|(s, e)| abs_col >= s && abs_col < e);
        let in_match = hl
            .match_cols
            .iter()
            .any(|&m| abs_col >= m && abs_col < m + hl.query_len);

        let base = hl
            .char_styles
            .and_then(|s| s.get(abs_col))
            .copied()
            .unwrap_or_default();
        let style = if in_selection {
            base.bg(SELECTION_BG)
        } else if in_match {
            base.bg(MATCH_BG).fg(Color::Black)
        } else if is_cursor_row {
            base.bg(CURRENT_LINE_BG)
        } else {
            base
        };

        if current_style != Some(style) {
            if !current.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current),
                    current_style.unwrap_or_default(),
                ));
            }
            current_style = Some(style);
        }
        current.push(*ch);
    }
    spans.push(Span::styled(current, current_style.unwrap_or_default()));

    Line::from(spans)
}

fn selection_cols_for_row(
    buf: &Buffer,
    row: usize,
    sel: Option<((usize, usize), (usize, usize))>,
) -> Option<(usize, usize)> {
    let (start, end) = sel?;
    if row < start.0 || row > end.0 {
        return None;
    }
    let line_len = buf.lines.get(row).map(|l| l.chars().count()).unwrap_or(0);
    let from = if row == start.0 { start.1 } else { 0 };
    let to = if row == end.0 { end.1 } else { line_len };
    Some((from, to))
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    // Highlight matches of whichever query is currently live (search bar,
    // or the "find" field while composing a replace).
    let highlight_query: Option<String> = match &app.mode {
        Mode::Search { query, .. } if !query.is_empty() => Some(query.clone()),
        Mode::Replace { find, .. } if !find.is_empty() => Some(find.clone()),
        _ => None,
    };

    // Syntax highlighting needs the parser to walk every line above the one
    // being rendered to track multi-line constructs correctly, so it works
    // off the whole buffer rather than just the visible slice. Borrowed
    // immutably alongside app.highlighter before we need &mut app.buffers.
    let char_styles: Option<Vec<Vec<Style>>> = {
        let buf = &app.buffers[app.active];
        let filename = buf.filename.as_ref().and_then(|p| p.to_str());
        app.highlighter.highlight(filename, &buf.lines)
    };

    let line_count = app.buffers[app.active].lines.len();
    let gutter_width = line_count.to_string().len().max(2) as u16 + 2;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gutter_width), Constraint::Min(0)])
        .split(area);
    let gutter_area = cols[0];
    let text_area = cols[1];
    app.last_text_area = text_area;

    let buf = &mut app.buffers[app.active];
    let visible_rows = text_area.height as usize;
    let visible_cols = text_area.width as usize;

    if buf.cursor_row < buf.row_offset {
        buf.row_offset = buf.cursor_row;
    } else if visible_rows > 0 && buf.cursor_row >= buf.row_offset + visible_rows {
        buf.row_offset = buf.cursor_row - visible_rows + 1;
    }

    if buf.cursor_col < buf.col_offset {
        buf.col_offset = buf.cursor_col;
    } else if visible_cols > 0 && buf.cursor_col >= buf.col_offset + visible_cols {
        buf.col_offset = buf.cursor_col - visible_cols + 1;
    }

    let end_row = (buf.row_offset + visible_rows).min(buf.lines.len());
    let selection = buf.selection_range();
    let matches: Vec<(usize, usize)> = highlight_query
        .as_deref()
        .map(|q| buf.find_all(q))
        .unwrap_or_default();
    let query_len = highlight_query.as_ref().map(|q| q.chars().count()).unwrap_or(0);
    let changed = buf.changed_lines();

    let mut gutter_lines = Vec::new();
    let mut text_lines = Vec::new();

    for row in buf.row_offset..end_row {
        let is_cursor_row = row == buf.cursor_row;
        let is_changed = changed.as_ref().is_some_and(|c| c[row]);

        let num_style = match (is_cursor_row, is_changed) {
            (true, true) => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            (true, false) => Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            (false, true) => Style::default().fg(Color::Yellow),
            (false, false) => Style::default().fg(DIM),
        };
        gutter_lines.push(Line::from(Span::styled(
            format!("{:>width$} ", row + 1, width = (gutter_width - 1) as usize),
            num_style,
        )));

        let sel_cols = selection_cols_for_row(buf, row, selection);
        let row_matches: Vec<usize> = matches
            .iter()
            .filter(|m| m.0 == row)
            .map(|m| m.1)
            .collect();
        let row_styles = char_styles.as_ref().map(|rows| rows[row].as_slice());
        let hl = RowHighlight {
            selection: sel_cols,
            match_cols: &row_matches,
            query_len,
            char_styles: row_styles,
        };

        text_lines.push(build_line_spans(
            &buf.lines[row],
            buf.col_offset,
            visible_cols,
            is_cursor_row,
            &hl,
        ));
    }

    frame.render_widget(Paragraph::new(gutter_lines), gutter_area);
    frame.render_widget(Paragraph::new(text_lines), text_area);

    let cursor_x = text_area.x + (buf.cursor_col - buf.col_offset) as u16;
    let cursor_y = text_area.y + (buf.cursor_row - buf.row_offset) as u16;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let hint = app.message.clone().unwrap_or_else(|| {
        " Ctrl+S save  Ctrl+Z/Y undo/redo  Ctrl+F find  Ctrl+H replace  Ctrl+T theme  \
          Ctrl+O open  Ctrl+PgUp/PgDn tabs  Ctrl+W close  Ctrl+Q quit"
            .to_string()
    });
    let buf = app.buf();
    let pos = format!(" Ln {}, Col {} ", buf.cursor_row + 1, buf.cursor_col + 1);

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

fn draw_prompt_popup(frame: &mut Frame, area: Rect, title: &str, input: &str) {
    let popup = centered_rect(50, 3, area);
    frame.render_widget(Clear, popup);
    let block = bordered(Style::default().fg(ACCENT)).title(title.to_string());
    frame.render_widget(Paragraph::new(format!("{input}_")).block(block), popup);
}

fn draw_search_popup(frame: &mut Frame, area: Rect, query: &str) {
    let popup = centered_rect(56, 3, area);
    frame.render_widget(Clear, popup);
    let block = bordered(Style::default().fg(ACCENT))
        .title(" Search  (Enter: next, Shift+Enter: prev, Esc: close) ");
    frame.render_widget(Paragraph::new(format!("{query}_")).block(block), popup);
}

fn draw_replace_popup(
    frame: &mut Frame,
    area: Rect,
    find: &str,
    replace: &str,
    editing_replace: bool,
) {
    let popup = centered_rect(58, 4, area);
    frame.render_widget(Clear, popup);
    let block = bordered(Style::default().fg(ACCENT)).title(
        " Replace all  (literal, case-sensitive; Tab: switch field, Enter: run, Esc: cancel) ",
    );
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let find_cursor = if !editing_replace { "_" } else { "" };
    let replace_cursor = if editing_replace { "_" } else { "" };
    let find_style = if !editing_replace {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let replace_style = if editing_replace {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };

    frame.render_widget(
        Paragraph::new(format!("Find:    {find}{find_cursor}")).style(find_style),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(format!("Replace: {replace}{replace_cursor}")).style(replace_style),
        rows[1],
    );
}

fn draw_confirm_popup(frame: &mut Frame, area: Rect, title: &str, body: &str) {
    let popup = centered_rect(50, 3, area);
    frame.render_widget(Clear, popup);
    let block = bordered(Style::default().fg(Color::Yellow)).title(title.to_string());
    frame.render_widget(Paragraph::new(body.to_string()).block(block), popup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::{backend::TestBackend, Terminal};

    // Regression test for a real bug: ratatui's default border symbols are
    // Unicode box-drawing characters, which come out as mojibake over a
    // terminal/serial console whose codepage isn't UTF-8. Popups must only
    // ever use the plain-ASCII border set.
    #[test]
    fn popups_never_render_unicode_box_drawing_characters() {
        let mut app = App::new(Vec::new()).unwrap();
        app.mode = crate::app::Mode::Open {
            input: "test".to_string(),
        };

        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| ui(frame, &mut app)).unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        for forbidden in ['\u{2500}', '\u{2502}', '\u{250c}', '\u{2510}', '\u{2514}', '\u{2518}'] {
            assert!(
                !rendered.contains(forbidden),
                "popup rendered a Unicode box-drawing character: {forbidden:?}"
            );
        }
        assert!(rendered.contains('+'), "expected ASCII border corners");
    }
}
