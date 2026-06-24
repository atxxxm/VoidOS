use std::{
    fs,
    io::{self, BufRead, Write},
    path::PathBuf,
};

const MAX_HISTORY: usize = 1000;

fn history_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".vsh_history")
}

pub struct History {
    entries: Vec<String>,
    index: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Self::load(),
            index: None,
        }
    }

    fn load() -> Vec<String> {
        let path = history_path();
        let Ok(file) = fs::File::open(&path) else {
            return Vec::new();
        };
        io::BufReader::new(file)
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| !l.trim().is_empty())
            .collect()
    }

    fn save(&self) {
        let path = history_path();
        let slice = if self.entries.len() > MAX_HISTORY {
            &self.entries[self.entries.len() - MAX_HISTORY..]
        } else {
            &self.entries
        };
        if let Ok(mut f) = fs::File::create(&path) {
            for line in slice {
                let _ = writeln!(f, "{}", line);
            }
        }
    }

    pub fn add_command(&mut self, command: &str) {
        let cmd = command.trim().to_string();
        if cmd.is_empty() {
            return;
        }
        if self.entries.last() == Some(&cmd) {
            return;
        }
        self.entries.push(cmd);
        self.save();
    }

    pub fn previous_command(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.index = Some(match self.index {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => self.entries.len() - 1,
        });
    }

    pub fn next_command(&mut self) {
        if let Some(i) = self.index {
            self.index = if i + 1 < self.entries.len() {
                Some(i + 1)
            } else {
                None
            };
        }
    }

    pub fn get_command(&self) -> String {
        self.index
            .map(|i| self.entries[i].clone())
            .unwrap_or_default()
    }

    pub fn zero_index(&mut self) {
        self.index = None;
    }

    pub fn all(&self) -> &[String] {
        &self.entries
    }
}
