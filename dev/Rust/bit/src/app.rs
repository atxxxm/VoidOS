use std::{fs, io, path::PathBuf};

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};

use crate::highlight::Highlighter;

// ---------------------------------------------------------------------------
// Undo/redo
// ---------------------------------------------------------------------------

// Consecutive edits of the same kind are coalesced into one undo step (so
// typing a word is one Ctrl+Z, not one per keystroke). Any non-edit action
// (cursor movement, save, ...) closes the current group.
#[derive(PartialEq, Eq, Clone, Copy)]
enum EditKind {
    Insert,
    Delete,
    Newline,
}

struct Snapshot {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

const MAX_UNDO_DEPTH: usize = 500;

pub enum SaveOutcome {
    Saved(PathBuf),
    NeedsName,
    Failed(io::Error),
}

impl SaveOutcome {
    fn needs_name(&self) -> bool {
        matches!(self, SaveOutcome::NeedsName)
    }
}

// ---------------------------------------------------------------------------
// Buffer: one open file (or unnamed scratch buffer) and its editing state.
// ---------------------------------------------------------------------------

pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub filename: Option<PathBuf>,
    pub modified: bool,
    pub selection_anchor: Option<(usize, usize)>,
    // Snapshot of `lines` as of the last save (or initial load) -- diffed
    // against the live buffer to mark changed lines in the gutter.
    saved_lines: Vec<String>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    last_edit_kind: Option<EditKind>,
}

impl Buffer {
    pub fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let (lines, filename) = match path {
            Some(p) if p.exists() => {
                let content = fs::read_to_string(&p)?;
                let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
                if lines.is_empty() {
                    lines.push(String::new());
                }
                (lines, Some(p))
            }
            Some(p) => (vec![String::new()], Some(p)),
            None => (vec![String::new()], None),
        };

        Ok(Self {
            saved_lines: lines.clone(),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            row_offset: 0,
            col_offset: 0,
            filename,
            modified: false,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit_kind: None,
        })
    }

    pub fn display_name(&self) -> String {
        self.filename
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "[New]".to_string())
    }

    // -- undo/redo ----------------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        }
    }

    fn record_undo(&mut self, kind: EditKind) {
        if self.last_edit_kind != Some(kind) {
            self.record_undo_boundary();
            self.last_edit_kind = Some(kind);
        }
    }

    // Always starts a fresh undo step (used for one-off batch edits: paste,
    // deleting a selection, replace-all -- these shouldn't merge with
    // whatever typing happened before or after them).
    fn record_undo_boundary(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > MAX_UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.last_edit_kind = None;
    }

    pub fn undo(&mut self) -> Option<&'static str> {
        let Some(snapshot) = self.undo_stack.pop() else {
            return Some("Nothing to undo");
        };
        self.redo_stack.push(self.snapshot());
        self.lines = snapshot.lines;
        self.cursor_row = snapshot.cursor_row;
        self.cursor_col = snapshot.cursor_col;
        self.last_edit_kind = None;
        self.selection_anchor = None;
        self.modified = true;
        None
    }

    pub fn redo(&mut self) -> Option<&'static str> {
        let Some(snapshot) = self.redo_stack.pop() else {
            return Some("Nothing to redo");
        };
        self.undo_stack.push(self.snapshot());
        self.lines = snapshot.lines;
        self.cursor_row = snapshot.cursor_row;
        self.cursor_col = snapshot.cursor_col;
        self.last_edit_kind = None;
        self.selection_anchor = None;
        self.modified = true;
        None
    }

    // -- cursor movement ------------------------------------------------

    pub fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].chars().count()
    }

    fn line_char_len(&self, row: usize) -> usize {
        self.lines[row].chars().count()
    }

    fn clamp_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    fn move_up_core(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    fn move_down_core(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    fn move_left_core(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
        }
    }

    fn move_right_core(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_home_core(&mut self) {
        self.cursor_col = 0;
    }

    fn move_end_core(&mut self) {
        self.cursor_col = self.current_line_len();
    }

    fn clear_selection_and_group(&mut self) {
        self.last_edit_kind = None;
        self.selection_anchor = None;
    }

    // Plain movement (no shift): clears any selection and closes the
    // current undo group.
    pub fn move_up(&mut self) {
        self.clear_selection_and_group();
        self.move_up_core();
    }

    pub fn move_down(&mut self) {
        self.clear_selection_and_group();
        self.move_down_core();
    }

    pub fn move_left(&mut self) {
        self.clear_selection_and_group();
        self.move_left_core();
    }

    pub fn move_right(&mut self) {
        self.clear_selection_and_group();
        self.move_right_core();
    }

    pub fn move_home(&mut self) {
        self.clear_selection_and_group();
        self.move_home_core();
    }

    pub fn move_end(&mut self) {
        self.clear_selection_and_group();
        self.move_end_core();
    }

    pub fn move_page(&mut self, amount: usize, down: bool) {
        self.clear_selection_and_group();
        for _ in 0..amount {
            if down {
                self.move_down_core();
            } else {
                self.move_up_core();
            }
        }
    }

    // Shift+movement: extends (or starts) a selection.
    fn ensure_anchor(&mut self) {
        self.last_edit_kind = None;
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some((self.cursor_row, self.cursor_col));
        }
    }

    pub fn extend_up(&mut self) {
        self.ensure_anchor();
        self.move_up_core();
    }

    pub fn extend_down(&mut self) {
        self.ensure_anchor();
        self.move_down_core();
    }

    pub fn extend_left(&mut self) {
        self.ensure_anchor();
        self.move_left_core();
    }

    pub fn extend_right(&mut self) {
        self.ensure_anchor();
        self.move_right_core();
    }

    pub fn extend_home(&mut self) {
        self.ensure_anchor();
        self.move_home_core();
    }

    pub fn extend_end(&mut self) {
        self.ensure_anchor();
        self.move_end_core();
    }

    pub fn select_all(&mut self) {
        self.last_edit_kind = None;
        self.selection_anchor = Some((0, 0));
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.current_line_len();
    }

    // -- selection ------------------------------------------------------

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.selection_anchor?;
        let cursor = (self.cursor_row, self.cursor_col);
        let range = if anchor <= cursor {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        };
        if range.0 == range.1 {
            None
        } else {
            Some(range)
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        if start.0 == end.0 {
            return Some(chars_slice(&self.lines[start.0], start.1, end.1));
        }
        let mut out = chars_slice(&self.lines[start.0], start.1, self.line_char_len(start.0));
        out.push('\n');
        for row in (start.0 + 1)..end.0 {
            out.push_str(&self.lines[row]);
            out.push('\n');
        }
        out.push_str(&chars_slice(&self.lines[end.0], 0, end.1));
        Some(out)
    }

    pub fn delete_selection(&mut self) {
        let Some((start, end)) = self.selection_range() else {
            return;
        };
        self.record_undo_boundary();
        if start.0 == end.0 {
            let idx_start = char_byte_index(&self.lines[start.0], start.1);
            let idx_end = char_byte_index(&self.lines[start.0], end.1);
            self.lines[start.0].replace_range(idx_start..idx_end, "");
        } else {
            let idx_start = char_byte_index(&self.lines[start.0], start.1);
            let end_line = self.lines[end.0].clone();
            let idx_end = char_byte_index(&end_line, end.1);
            let tail = end_line[idx_end..].to_string();
            self.lines[start.0].truncate(idx_start);
            self.lines[start.0].push_str(&tail);
            self.lines.drain(start.0 + 1..=end.0);
        }
        self.cursor_row = start.0;
        self.cursor_col = start.1;
        self.selection_anchor = None;
        self.modified = true;
    }

    // -- editing ----------------------------------------------------------

    pub fn insert_char(&mut self, c: char) {
        self.record_undo(EditKind::Insert);
        let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
        self.lines[self.cursor_row].insert(idx, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.record_undo(EditKind::Newline);
        let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
        let rest = self.lines[self.cursor_row].split_off(idx);
        // Auto-indent: carry over the leading whitespace of the line we're
        // leaving, so blocks of code stay aligned without retyping tabs.
        let indent: String = self.lines[self.cursor_row]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let new_line = format!("{indent}{rest}");
        self.lines.insert(self.cursor_row + 1, new_line);
        self.cursor_row += 1;
        self.cursor_col = indent.chars().count();
        self.modified = true;
    }

    // Inserts (possibly multi-line) text at the cursor -- used for paste.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.record_undo_boundary();

        let mut parts = text.split('\n');
        let first = parts.next().unwrap_or("");
        let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
        self.lines[self.cursor_row].insert_str(idx, first);
        self.cursor_col += first.chars().count();

        let rest: Vec<&str> = parts.collect();
        if !rest.is_empty() {
            let split_idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
            let tail = self.lines[self.cursor_row].split_off(split_idx);
            let mut row = self.cursor_row;
            let last = rest.len() - 1;
            let mut final_col = 0;
            for (i, part) in rest.into_iter().enumerate() {
                row += 1;
                let mut line = part.to_string();
                if i == last {
                    final_col = line.chars().count();
                    line.push_str(&tail);
                }
                self.lines.insert(row, line);
            }
            self.cursor_row = row;
            self.cursor_col = final_col;
        }
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        self.record_undo(EditKind::Delete);
        if self.cursor_col > 0 {
            let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col - 1);
            self.lines[self.cursor_row].remove(idx);
            self.cursor_col -= 1;
            self.modified = true;
        } else if self.cursor_row > 0 {
            let line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
            self.lines[self.cursor_row].push_str(&line);
            self.modified = true;
        }
    }

    pub fn delete_forward(&mut self) {
        self.record_undo(EditKind::Delete);
        let len = self.current_line_len();
        if self.cursor_col < len {
            let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].remove(idx);
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.modified = true;
        }
    }

    // -- search / replace -------------------------------------------------

    // All (row, col) positions where `query` matches, case-insensitively,
    // in document order. Recomputed on demand -- simple and fast enough
    // for editor-sized files, no need to maintain a search index.
    pub fn find_all(&self, query: &str) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return Vec::new();
        }
        let needle = query.to_lowercase();
        let mut out = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            let lower = line.to_lowercase();
            let mut start_byte = 0;
            while let Some(pos) = lower[start_byte..].find(&needle) {
                let abs_byte = start_byte + pos;
                let col = lower[..abs_byte].chars().count();
                out.push((row, col));
                start_byte = abs_byte + needle.len();
            }
        }
        out
    }

    // Literal, case-sensitive replace-all (unlike the case-insensitive
    // search) -- keeps "what you typed is what gets matched" unambiguous
    // without needing case-preservation rules for replacements.
    pub fn replace_all(&mut self, find: &str, replace: &str) -> usize {
        if find.is_empty() {
            return 0;
        }
        self.record_undo_boundary();
        let mut count = 0;
        for line in self.lines.iter_mut() {
            let occurrences = line.matches(find).count();
            if occurrences > 0 {
                *line = line.replace(find, replace);
                count += occurrences;
            }
        }
        if count > 0 {
            self.modified = true;
        }
        self.clamp_col();
        count
    }

    // -- save -------------------------------------------------------------

    pub fn save(&mut self) -> SaveOutcome {
        let Some(path) = self.filename.clone() else {
            return SaveOutcome::NeedsName;
        };
        match fs::write(&path, self.lines.join("\n")) {
            Ok(()) => {
                self.modified = false;
                self.saved_lines = self.lines.clone();
                SaveOutcome::Saved(path)
            }
            Err(e) => SaveOutcome::Failed(e),
        }
    }

    pub fn save_as(&mut self, name: String) -> io::Result<PathBuf> {
        let path = PathBuf::from(name);
        fs::write(&path, self.lines.join("\n"))?;
        self.modified = false;
        self.saved_lines = self.lines.clone();
        self.filename = Some(path.clone());
        Ok(path)
    }

    // -- gutter change indicator -------------------------------------------

    // Which currently-existing lines differ from the last-saved state, via
    // a plain LCS diff (correctly tracks inserted lines instead of naively
    // flagging everything below one as "changed"). Skipped above a size
    // guard to bound worst-case O(n*m) time/space on huge files.
    pub fn changed_lines(&self) -> Option<Vec<bool>> {
        let n = self.saved_lines.len();
        let m = self.lines.len();
        if n.saturating_mul(m) > 4_000_000 {
            return None;
        }
        Some(diff_added_lines(&self.saved_lines, &self.lines))
    }
}

fn diff_added_lines(saved: &[String], current: &[String]) -> Vec<bool> {
    let n = saved.len();
    let m = current.len();
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if saved[i] == current[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut changed = vec![true; m];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if saved[i] == current[j] {
            changed[j] = false;
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    changed
}

fn chars_slice(s: &str, from: usize, to: usize) -> String {
    let start = char_byte_index(s, from);
    let end = char_byte_index(s, to);
    s[start..end].to_string()
}

fn char_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn next_match(
    matches: &[(usize, usize)],
    from: (usize, usize),
    strict: bool,
) -> Option<(usize, usize)> {
    matches
        .iter()
        .find(|&&m| if strict { m > from } else { m >= from })
        .copied()
        .or_else(|| matches.first().copied())
}

fn prev_match(matches: &[(usize, usize)], from: (usize, usize)) -> Option<(usize, usize)> {
    matches
        .iter()
        .rev()
        .find(|&&m| m < from)
        .copied()
        .or_else(|| matches.last().copied())
}

// ---------------------------------------------------------------------------
// App: window chrome around one or more buffers (tabs), plus state that
// doesn't belong to any single buffer (mode, clipboard).
// ---------------------------------------------------------------------------

pub enum Mode {
    Normal,
    SaveAs {
        input: String,
    },
    ConfirmQuit,
    ConfirmCloseTab,
    Search {
        query: String,
        origin: (usize, usize),
    },
    Replace {
        find: String,
        replace: String,
        editing_replace: bool,
    },
    Open {
        input: String,
    },
}

pub struct App {
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub mode: Mode,
    pub message: Option<String>,
    pub should_quit: bool,
    pub clipboard: Option<String>,
    pub highlighter: Highlighter,
    // The text area's screen rect as of the last render, used to translate
    // mouse-click coordinates back into buffer (row, col). Zeroed until the
    // first draw; a click before that just misses the bounds check below.
    pub last_text_area: Rect,
}

impl App {
    pub fn new(paths: Vec<PathBuf>) -> io::Result<Self> {
        let buffers = if paths.is_empty() {
            vec![Buffer::open(None)?]
        } else {
            paths
                .into_iter()
                .map(|p| Buffer::open(Some(p)))
                .collect::<io::Result<Vec<_>>>()?
        };

        Ok(Self {
            buffers,
            active: 0,
            mode: Mode::Normal,
            message: None,
            should_quit: false,
            clipboard: None,
            highlighter: Highlighter::new(),
            last_text_area: Rect::new(0, 0, 0, 0),
        })
    }

    pub fn buf(&self) -> &Buffer {
        &self.buffers[self.active]
    }

    pub fn buf_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.active]
    }

    pub fn next_tab(&mut self) {
        self.active = (self.active + 1) % self.buffers.len();
    }

    pub fn prev_tab(&mut self) {
        self.active = (self.active + self.buffers.len() - 1) % self.buffers.len();
    }

    fn new_buffer(&mut self) {
        self.buffers
            .push(Buffer::open(None).expect("in-memory buffer cannot fail"));
        self.active = self.buffers.len() - 1;
    }

    fn copy(&mut self) {
        match self.buf().selected_text() {
            Some(text) => {
                self.clipboard = Some(text);
                self.message = Some("Copied".to_string());
            }
            None => self.message = Some("Nothing selected".to_string()),
        }
    }

    fn cut(&mut self) {
        match self.buf().selected_text() {
            Some(text) => {
                self.clipboard = Some(text);
                self.buf_mut().delete_selection();
                self.message = Some("Cut".to_string());
            }
            None => self.message = Some("Nothing selected".to_string()),
        }
    }

    fn paste(&mut self) {
        let Some(text) = self.clipboard.clone() else {
            self.message = Some("Clipboard is empty".to_string());
            return;
        };
        if self.buf().selection_anchor.is_some() {
            self.buf_mut().delete_selection();
        }
        self.buf_mut().insert_text(&text);
    }

    fn jump_to_match(&mut self, query: &str, from: (usize, usize), strict: bool) {
        let matches = self.buf().find_all(query);
        if matches.is_empty() {
            self.message = Some("No matches".to_string());
            return;
        }
        self.message = None;
        if let Some((row, col)) = next_match(&matches, from, strict) {
            self.buf_mut().cursor_row = row;
            self.buf_mut().cursor_col = col;
        }
    }

    fn jump_to_prev_match(&mut self, query: &str) {
        let matches = self.buf().find_all(query);
        if matches.is_empty() {
            self.message = Some("No matches".to_string());
            return;
        }
        self.message = None;
        let from = (self.buf().cursor_row, self.buf().cursor_col);
        if let Some((row, col)) = prev_match(&matches, from) {
            self.buf_mut().cursor_row = row;
            self.buf_mut().cursor_col = col;
        }
    }

    fn save_active(&mut self) {
        match self.buf_mut().save() {
            SaveOutcome::Saved(path) => self.message = Some(format!("Saved: {}", path.display())),
            SaveOutcome::NeedsName => self.mode = Mode::SaveAs { input: String::new() },
            SaveOutcome::Failed(e) => self.message = Some(format!("Save failed: {e}")),
        }
    }

    fn request_quit(&mut self) {
        if self.buffers.iter().any(|b| b.modified) {
            self.mode = Mode::ConfirmQuit;
        } else {
            self.should_quit = true;
        }
    }

    fn request_close_active(&mut self) {
        if self.buf().modified {
            self.mode = Mode::ConfirmCloseTab;
        } else {
            self.close_active_now();
        }
    }

    fn close_active_now(&mut self) {
        if self.buffers.len() == 1 {
            self.should_quit = true;
        } else {
            self.buffers.remove(self.active);
            if self.active >= self.buffers.len() {
                self.active = self.buffers.len() - 1;
            }
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Some(i) = self
            .buffers
            .iter()
            .position(|b| b.filename.as_deref() == Some(path.as_path()))
        {
            self.active = i;
            return;
        }
        match Buffer::open(Some(path)) {
            Ok(buf) => {
                self.buffers.push(buf);
                self.active = self.buffers.len() - 1;
            }
            Err(e) => self.message = Some(format!("Open failed: {e}")),
        }
    }

    // Only click-to-position and wheel-scroll -- drag-to-select would need
    // to track mouse-down state across events, which isn't worth the extra
    // complexity for a terminal editor's mouse support.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if !matches!(self.mode, Mode::Normal) {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.click_at(mouse.column, mouse.row),
            MouseEventKind::ScrollUp => self.buf_mut().move_page(3, false),
            MouseEventKind::ScrollDown => self.buf_mut().move_page(3, true),
            _ => {}
        }
    }

    fn click_at(&mut self, x: u16, y: u16) {
        let area = self.last_text_area;
        if area.width == 0
            || area.height == 0
            || x < area.x
            || y < area.y
            || x >= area.x + area.width
            || y >= area.y + area.height
        {
            return;
        }
        let buf = self.buf_mut();
        let row = (buf.row_offset + (y - area.y) as usize).min(buf.lines.len() - 1);
        let col = buf.col_offset + (x - area.x) as usize;
        buf.selection_anchor = None;
        buf.cursor_row = row;
        buf.cursor_col = col.min(buf.current_line_len());
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::SaveAs { .. } => self.handle_save_as_key(key),
            Mode::ConfirmQuit => self.handle_confirm_quit_key(key),
            Mode::ConfirmCloseTab => self.handle_confirm_close_tab_key(key),
            Mode::Search { .. } => self.handle_search_key(key),
            Mode::Replace { .. } => self.handle_replace_key(key),
            Mode::Open { .. } => self.handle_open_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        self.message = None;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        if ctrl {
            match key.code {
                KeyCode::Char('s') => self.save_active(),
                KeyCode::Char('z') => {
                    if let Some(msg) = self.buf_mut().undo() {
                        self.message = Some(msg.to_string());
                    }
                }
                KeyCode::Char('y') => {
                    if let Some(msg) = self.buf_mut().redo() {
                        self.message = Some(msg.to_string());
                    }
                }
                KeyCode::Char('a') => self.buf_mut().select_all(),
                KeyCode::Char('c') => self.copy(),
                KeyCode::Char('x') => self.cut(),
                KeyCode::Char('v') => self.paste(),
                KeyCode::Char('f') => {
                    let origin = (self.buf().cursor_row, self.buf().cursor_col);
                    self.mode = Mode::Search {
                        query: String::new(),
                        origin,
                    };
                }
                KeyCode::Char('h') => {
                    self.mode = Mode::Replace {
                        find: String::new(),
                        replace: String::new(),
                        editing_replace: false,
                    };
                }
                KeyCode::Char('o') => self.mode = Mode::Open { input: String::new() },
                KeyCode::Char('n') => self.new_buffer(),
                KeyCode::Char('w') => self.request_close_active(),
                KeyCode::Char('t') => {
                    self.highlighter.cycle_theme();
                    self.message = Some(format!("Theme: {}", self.highlighter.theme_name()));
                }
                KeyCode::PageDown => self.next_tab(),
                KeyCode::PageUp => self.prev_tab(),
                KeyCode::Char('q') => self.request_quit(),
                _ => {}
            }
            return;
        }

        match (key.code, shift) {
            (KeyCode::Left, true) => self.buf_mut().extend_left(),
            (KeyCode::Right, true) => self.buf_mut().extend_right(),
            (KeyCode::Up, true) => self.buf_mut().extend_up(),
            (KeyCode::Down, true) => self.buf_mut().extend_down(),
            (KeyCode::Home, true) => self.buf_mut().extend_home(),
            (KeyCode::End, true) => self.buf_mut().extend_end(),
            (KeyCode::Left, false) => self.buf_mut().move_left(),
            (KeyCode::Right, false) => self.buf_mut().move_right(),
            (KeyCode::Up, false) => self.buf_mut().move_up(),
            (KeyCode::Down, false) => self.buf_mut().move_down(),
            (KeyCode::Home, false) => self.buf_mut().move_home(),
            (KeyCode::End, false) => self.buf_mut().move_end(),
            (KeyCode::PageUp, _) => self.buf_mut().move_page(20, false),
            (KeyCode::PageDown, _) => self.buf_mut().move_page(20, true),
            (KeyCode::Backspace, _) => {
                if self.buf().selection_anchor.is_some() {
                    self.buf_mut().delete_selection();
                } else {
                    self.buf_mut().backspace();
                }
            }
            (KeyCode::Delete, _) => {
                if self.buf().selection_anchor.is_some() {
                    self.buf_mut().delete_selection();
                } else {
                    self.buf_mut().delete_forward();
                }
            }
            (KeyCode::Enter, _) => {
                if self.buf().selection_anchor.is_some() {
                    self.buf_mut().delete_selection();
                }
                self.buf_mut().insert_newline();
            }
            (KeyCode::Tab, _) => {
                if self.buf().selection_anchor.is_some() {
                    self.buf_mut().delete_selection();
                }
                for _ in 0..4 {
                    self.buf_mut().insert_char(' ');
                }
            }
            (KeyCode::Char(c), _) => {
                if self.buf().selection_anchor.is_some() {
                    self.buf_mut().delete_selection();
                }
                self.buf_mut().insert_char(c);
            }
            _ => {}
        }
    }

    fn handle_save_as_key(&mut self, key: KeyEvent) {
        let Mode::SaveAs { input } = &mut self.mode else {
            return;
        };

        match key.code {
            KeyCode::Enter => {
                let name = input.trim().to_string();
                self.mode = Mode::Normal;
                if name.is_empty() {
                    self.message = Some("Save cancelled: empty filename".to_string());
                } else {
                    match self.buf_mut().save_as(name) {
                        Ok(path) => self.message = Some(format!("Saved: {}", path.display())),
                        Err(e) => self.message = Some(format!("Save failed: {e}")),
                    }
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.message = Some("Save cancelled".to_string());
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => input.push(c),
            _ => {}
        }
    }

    fn handle_confirm_quit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let original_active = self.active;
                let mut skipped = 0;
                for i in 0..self.buffers.len() {
                    if self.buffers[i].modified {
                        self.active = i;
                        if self.buf_mut().save().needs_name() {
                            skipped += 1;
                        }
                    }
                }
                self.active = original_active;

                if skipped > 0 {
                    self.mode = Mode::Normal;
                    self.message = Some(format!(
                        "{skipped} unnamed buffer(s) not saved -- Ctrl+S them first, then quit again"
                    ));
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => self.should_quit = true,
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_confirm_close_tab_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match self.buf_mut().save() {
                SaveOutcome::Saved(_) => {
                    self.mode = Mode::Normal;
                    self.close_active_now();
                }
                SaveOutcome::NeedsName => {
                    self.mode = Mode::Normal;
                    self.message = Some("No filename yet -- save with Ctrl+S first".to_string());
                }
                SaveOutcome::Failed(e) => {
                    self.mode = Mode::Normal;
                    self.message = Some(format!("Save failed: {e}"));
                }
            },
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.mode = Mode::Normal;
                self.close_active_now();
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        // Mutate the query in its own scope so the borrow of self.mode ends
        // before we call other &mut self methods below.
        let (query_now, origin) = {
            let Mode::Search { query, origin } = &mut self.mode else {
                return;
            };
            match key.code {
                KeyCode::Backspace => {
                    query.pop();
                }
                KeyCode::Char(c) => query.push(c),
                _ => {}
            }
            (query.clone(), *origin)
        };

        match key.code {
            KeyCode::Enter => {
                if shift {
                    self.jump_to_prev_match(&query_now);
                } else {
                    let from = (self.buf().cursor_row, self.buf().cursor_col);
                    self.jump_to_match(&query_now, from, true);
                }
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace | KeyCode::Char(_) => {
                if query_now.is_empty() {
                    self.buf_mut().cursor_row = origin.0;
                    self.buf_mut().cursor_col = origin.1;
                    self.message = None;
                } else {
                    self.jump_to_match(&query_now, origin, false);
                }
            }
            _ => {}
        }
    }

    fn handle_replace_key(&mut self, key: KeyEvent) {
        let Mode::Replace {
            find,
            replace,
            editing_replace,
        } = &mut self.mode
        else {
            return;
        };

        match key.code {
            KeyCode::Tab => *editing_replace = !*editing_replace,
            KeyCode::Enter => {
                let (find, replace) = (find.clone(), replace.clone());
                self.mode = Mode::Normal;
                if find.is_empty() {
                    self.message = Some("Replace cancelled: empty search text".to_string());
                } else {
                    let count = self.buf_mut().replace_all(&find, &replace);
                    self.message = Some(format!("Replaced {count} occurrence(s)"));
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.message = Some("Replace cancelled".to_string());
            }
            KeyCode::Backspace => {
                if *editing_replace {
                    replace.pop();
                } else {
                    find.pop();
                }
            }
            KeyCode::Char(c) => {
                if *editing_replace {
                    replace.push(c);
                } else {
                    find.push(c);
                }
            }
            _ => {}
        }
    }

    fn handle_open_key(&mut self, key: KeyEvent) {
        let Mode::Open { input } = &mut self.mode else {
            return;
        };

        match key.code {
            KeyCode::Enter => {
                let name = input.trim().to_string();
                self.mode = Mode::Normal;
                if name.is_empty() {
                    self.message = Some("Open cancelled: empty filename".to_string());
                } else {
                    self.open_path(PathBuf::from(name));
                }
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => input.push(c),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_buf() -> Buffer {
        Buffer::open(None).unwrap()
    }

    fn new_app() -> App {
        App::new(Vec::new()).unwrap()
    }

    // -- undo/redo --------------------------------------------------------

    #[test]
    fn undo_reverts_a_run_of_typed_chars_in_one_step() {
        let mut buf = new_buf();
        for c in "hi".chars() {
            buf.insert_char(c);
        }
        assert_eq!(buf.lines, vec!["hi".to_string()]);

        buf.undo();
        assert_eq!(buf.lines, vec!["".to_string()]);
        assert_eq!(buf.cursor_col, 0);
    }

    #[test]
    fn cursor_movement_breaks_the_undo_group() {
        let mut buf = new_buf();
        buf.insert_char('a');
        buf.move_left();
        buf.insert_char('b');
        assert_eq!(buf.lines, vec!["ba".to_string()]);

        buf.undo();
        assert_eq!(buf.lines, vec!["a".to_string()]);
        buf.undo();
        assert_eq!(buf.lines, vec!["".to_string()]);
    }

    #[test]
    fn redo_reapplies_an_undone_edit() {
        let mut buf = new_buf();
        buf.insert_char('x');
        buf.undo();
        assert_eq!(buf.lines, vec!["".to_string()]);

        buf.redo();
        assert_eq!(buf.lines, vec!["x".to_string()]);
    }

    #[test]
    fn editing_after_undo_clears_the_redo_stack() {
        let mut buf = new_buf();
        buf.insert_char('a');
        buf.undo();
        buf.insert_char('b');

        buf.redo();
        assert_eq!(buf.lines, vec!["b".to_string()]);
    }

    // -- selection / copy-paste -------------------------------------------

    #[test]
    fn selection_across_lines_extracts_expected_text() {
        let mut buf = new_buf();
        buf.insert_text("hello\nworld");
        buf.cursor_row = 0;
        buf.cursor_col = 3;
        buf.selection_anchor = Some((0, 3));
        buf.cursor_row = 1;
        buf.cursor_col = 3;
        assert_eq!(buf.selected_text(), Some("lo\nwor".to_string()));
    }

    #[test]
    fn delete_selection_removes_the_range_and_joins_lines() {
        let mut buf = new_buf();
        buf.insert_text("hello\nworld");
        buf.selection_anchor = Some((0, 3));
        buf.cursor_row = 1;
        buf.cursor_col = 3;
        buf.delete_selection();
        assert_eq!(buf.lines, vec!["helld".to_string()]);
        assert_eq!((buf.cursor_row, buf.cursor_col), (0, 3));
    }

    #[test]
    fn multiline_paste_splits_correctly_around_cursor() {
        let mut buf = new_buf();
        buf.insert_text("()");
        buf.cursor_col = 1;
        buf.insert_text("a\nb\nc");
        assert_eq!(
            buf.lines,
            vec!["(a".to_string(), "b".to_string(), "c)".to_string()]
        );
        assert_eq!((buf.cursor_row, buf.cursor_col), (2, 1));
    }

    #[test]
    fn cross_buffer_clipboard_copy_and_paste() {
        let mut app = new_app();
        app.buf_mut().insert_text("secret");
        app.buf_mut().selection_anchor = Some((0, 0));
        app.buf_mut().cursor_col = 6;
        app.copy();
        assert_eq!(app.clipboard.as_deref(), Some("secret"));

        app.new_buffer();
        assert_eq!(app.active, 1);
        app.paste();
        assert_eq!(app.buf().lines, vec!["secret".to_string()]);
    }

    // -- search / replace --------------------------------------------------

    #[test]
    fn find_all_is_case_insensitive_and_in_document_order() {
        let mut buf = new_buf();
        buf.insert_text("Foo bar\nfoofoo");
        assert_eq!(buf.find_all("foo"), vec![(0, 0), (1, 0), (1, 3)]);
    }

    #[test]
    fn replace_all_is_literal_and_case_sensitive() {
        let mut buf = new_buf();
        buf.insert_text("cat Cat cat");
        let count = buf.replace_all("cat", "dog");
        assert_eq!(count, 2);
        assert_eq!(buf.lines, vec!["dog Cat dog".to_string()]);
    }

    // -- tabs ---------------------------------------------------------------

    #[test]
    fn new_buffer_and_tab_switching() {
        let mut app = new_app();
        app.new_buffer();
        app.new_buffer();
        assert_eq!(app.buffers.len(), 3);
        assert_eq!(app.active, 2);

        app.next_tab();
        assert_eq!(app.active, 0);
        app.prev_tab();
        assert_eq!(app.active, 2);
    }

    #[test]
    fn closing_a_tab_removes_it_and_keeps_others() {
        let mut app = new_app();
        app.new_buffer();
        app.buf_mut().insert_char('x');
        app.active = 0;

        app.request_close_active();
        assert_eq!(app.buffers.len(), 1);
        assert_eq!(app.buf().lines, vec!["x".to_string()]);
        assert!(!app.should_quit);
    }

    #[test]
    fn closing_the_last_tab_quits() {
        let mut app = new_app();
        app.request_close_active();
        assert!(app.should_quit);
    }

    // -- auto-indent ---------------------------------------------------------

    #[test]
    fn insert_newline_carries_over_leading_indentation() {
        let mut buf = new_buf();
        buf.insert_text("    if x {");
        buf.insert_newline();
        assert_eq!(buf.lines[1], "    ");
        assert_eq!(buf.cursor_col, 4);
    }

    // -- gutter change indicator ----------------------------------------------

    #[test]
    fn changed_lines_tracks_insertions_without_flagging_unrelated_lines() {
        let mut buf = new_buf();
        buf.lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        buf.saved_lines = buf.lines.clone();

        // Insert a new line at the top -- "b" and "c" should still read as
        // unchanged even though their row index shifted by one.
        buf.lines.insert(0, "new".to_string());

        let changed = buf.changed_lines().unwrap();
        assert_eq!(changed, vec![true, false, false, false]);
    }

    // -- mouse ----------------------------------------------------------------

    #[test]
    fn mouse_click_moves_cursor_to_clicked_position() {
        let mut app = new_app();
        app.buf_mut().insert_text("hello\nworld");
        app.last_text_area = Rect::new(5, 2, 20, 10);

        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 8,
            row: 3,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!((app.buf().cursor_row, app.buf().cursor_col), (1, 3));
    }

    #[test]
    fn mouse_click_outside_text_area_is_ignored() {
        let mut app = new_app();
        app.buf_mut().insert_text("hello");
        let cursor_before = (app.buf().cursor_row, app.buf().cursor_col);
        app.last_text_area = Rect::new(5, 2, 20, 10);

        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!((app.buf().cursor_row, app.buf().cursor_col), cursor_before);
    }
}
