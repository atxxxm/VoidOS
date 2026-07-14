use std::{fs, io, path::PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Mode {
    Normal,
    SaveAs { input: String },
    ConfirmQuit,
}

pub struct App {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub filename: Option<PathBuf>,
    pub modified: bool,
    pub message: Option<String>,
    pub mode: Mode,
    pub should_quit: bool,
}

impl App {
    pub fn new(path: Option<PathBuf>) -> io::Result<Self> {
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
            lines,
            cursor_row: 0,
            cursor_col: 0,
            row_offset: 0,
            col_offset: 0,
            filename,
            modified: false,
            message: None,
            mode: Mode::Normal,
            should_quit: false,
        })
    }

    pub fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].chars().count()
    }

    fn clamp_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
        }
    }

    pub fn move_right(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.current_line_len();
    }

    pub fn move_page(&mut self, amount: usize, down: bool) {
        for _ in 0..amount {
            if down {
                self.move_down();
            } else {
                self.move_up();
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
        self.lines[self.cursor_row].insert(idx, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        let idx = char_byte_index(&self.lines[self.cursor_row], self.cursor_col);
        let rest = self.lines[self.cursor_row].split_off(idx);
        self.lines.insert(self.cursor_row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.modified = true;
    }

    pub fn backspace(&mut self) {
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

    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = self.filename.clone() else {
            self.mode = Mode::SaveAs { input: String::new() };
            return Ok(());
        };
        fs::write(&path, self.lines.join("\n"))?;
        self.modified = false;
        self.message = Some(format!("Saved: {}", path.display()));
        Ok(())
    }

    pub fn save_as(&mut self, name: String) -> io::Result<()> {
        let path = PathBuf::from(name);
        fs::write(&path, self.lines.join("\n"))?;
        self.modified = false;
        self.message = Some(format!("Saved: {}", path.display()));
        self.filename = Some(path);
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::SaveAs { .. } => self.handle_save_as_key(key),
            Mode::ConfirmQuit => self.handle_confirm_quit_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        self.message = None;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match (key.code, ctrl) {
            (KeyCode::Char('s'), true) => {
                if let Err(e) = self.save() {
                    self.message = Some(format!("Save failed: {e}"));
                }
            }
            (KeyCode::Char('q'), true) => {
                if self.modified {
                    self.mode = Mode::ConfirmQuit;
                } else {
                    self.should_quit = true;
                }
            }
            (KeyCode::Up, _) => self.move_up(),
            (KeyCode::Down, _) => self.move_down(),
            (KeyCode::Left, _) => self.move_left(),
            (KeyCode::Right, _) => self.move_right(),
            (KeyCode::Home, _) => self.move_home(),
            (KeyCode::End, _) => self.move_end(),
            (KeyCode::PageUp, _) => self.move_page(20, false),
            (KeyCode::PageDown, _) => self.move_page(20, true),
            (KeyCode::Backspace, _) => self.backspace(),
            (KeyCode::Delete, _) => self.delete_forward(),
            (KeyCode::Enter, _) => self.insert_newline(),
            (KeyCode::Tab, _) => {
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            (KeyCode::Char(c), false) => self.insert_char(c),
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
                } else if let Err(e) = self.save_as(name) {
                    self.message = Some(format!("Save failed: {e}"));
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
                if self.filename.is_none() {
                    self.mode = Mode::Normal;
                    self.message = Some("No filename yet -- save with Ctrl+S first".to_string());
                    return;
                }
                match self.save() {
                    Ok(()) => self.should_quit = true,
                    Err(e) => {
                        self.mode = Mode::Normal;
                        self.message = Some(format!("Save failed: {e}"));
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => self.should_quit = true,
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }
}

fn char_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
