use std::{fs, io::{Stdout, Write}, path::Path};
use colored::Colorize;
use crossterm::{execute, terminal};
use crate::utils::get_current_path;
use anyhow;


pub struct Prompt {
    pub prompt: String,
    pub cursor_pos: usize,
    cursor_symbol: char,
    username: String,
    stdout: Stdout,
}

impl Prompt {
    pub fn new(username: String, stdout: Stdout, cursor_symbol: char) -> Self {
        Self { prompt: String::new(), cursor_pos: 0, cursor_symbol, username, stdout }
    }

    // Get body of prompt
    fn get_body(&self) -> String {
        let cur_path = get_current_path();
        format!("{}$ [{}] > ", &self.username.red(), cur_path)
    }

    // Working with text //

    // Add char to prompt
    pub fn add(&mut self, ch: char) {
        let byte_idx = self.get_byte_idx();
        self.prompt.insert(byte_idx, ch);  
    }

    // Delete char from prompt
    pub fn delete(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            let byte_idx = self.get_byte_idx();
            self.prompt.remove(byte_idx);
        }
    }

    // Default prompt
    pub fn default(&mut self) {
        self.prompt.clear();
        self.cursor_pos = 0;
    }

    // Get byte index
    fn get_byte_idx(&self) -> usize {
        self.prompt.char_indices().nth(self.cursor_pos).map(|(i, _)| i).unwrap_or(self.prompt.len())
    }


    // Work with cursor //

    // Cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    // Cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.prompt.len() {
            self.cursor_pos += 1;
        }
    }

    // Cursor start
    pub fn cursor_start(&mut self) {
        self.cursor_pos = 0;
    }

    // Cursor end
    pub fn cursor_end(&mut self) {
        self.cursor_pos = self.prompt.len();
    }

    // Output //

    // Print prompt
    pub fn print(&mut self) -> anyhow::Result<()> {
        execute!(self.stdout, terminal::Clear(terminal::ClearType::CurrentLine))?;
        print!("\r{}", self.get_body());

        for (i, ch) in self.prompt.chars().enumerate() {
            if i == self.cursor_pos {
                print!("{}", self.cursor_symbol);
            }

            print!("{}", ch);
        }

        if self.cursor_pos == self.prompt.len() {
            print!("{}", self.cursor_symbol);
        }

        self.stdout.flush().unwrap();
        Ok(())
    }

    // Autocomplete //

    pub fn autocomplete(&mut self) -> anyhow::Result<()> {
        if self.prompt.is_empty() {
            return Ok(());
        }

        let (start, end, target) = self.get_current_word();

        if target.is_empty() {
            return Ok(());
        }

        let storage = if target.contains('/') {
            self.complete_path(&target)?
        } else {
            let mut st = self.check_current_dir_files(&target)?;
            let bin_vec = self.check_path_file(&target)?;
            st.extend(bin_vec);
            st.sort();
            st
        };

        if storage.is_empty() {
            return Ok(());
        }

        if storage.len() == 1 {
            self.replace_range(start, end, &storage[0]);
            self.cursor_pos = start + storage[0].len();
            return Ok(());
        }

        let prefix = self.common_prefix(&storage);

        if prefix.len() > target.len() {
            self.replace_range(start, end, &prefix);
            self.cursor_pos = start + prefix.len();
            return Ok(());
        }

        println!("\r\n{}", storage.join(" "));

        Ok(())

    }

    // View files in the current directory
    fn check_current_dir_files(&self, target: &str) -> anyhow::Result<Vec<String>> {
        let mut storage: Vec<String> = Vec::new();

        for file in fs::read_dir(".")? {
            let file = file?;
            let filename = file.file_name().display().to_string();

            if filename.starts_with(target) {
                storage.push(filename);
            }
        }

        Ok(storage)
    }

    // View files in the PATH
    fn check_path_file(&self, target: &str) -> anyhow::Result<Vec<String>> {
        let path = std::env::var("PATH")?;

        let mut bin_vec: Vec<String> = Vec::new();

        for dir in path.split(':') {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let filename = entry.file_name().display().to_string();

                if filename.starts_with(target) {
                    bin_vec.push(filename);
                }
            }
        }

        Ok(bin_vec)
    }

    // Get current word
    fn get_current_word(&self) -> (usize, usize, String) {
        let chars: Vec<char> = self.prompt.chars().collect();
        let mut start = self.cursor_pos;
        let mut end = self.cursor_pos;

        while start > 0 && chars[start - 1] != ' ' {
            start -= 1;
        }

        while end < chars.len() && chars[end] != ' ' {
            end += 1;
        }

        let word: String = chars[start..end].iter().collect();
        (start, end, word)
    }

    // Replace range in prompt
    fn replace_range(&mut self, start: usize, end: usize, new: &str) {
        self.prompt.replace_range(start..end, new);
    }

    // Get common prefix
    fn common_prefix(&self, list: &Vec<String>) -> String {
        if list.is_empty() {
            return String::new();
        }

        let mut prefix = list[0].clone();

        for item in list.iter().skip(1) {
            let mut new = String::new();

            for (a, b) in prefix.chars().zip(item.chars()) {
                if a == b {
                    new.push(a);
                } else {
                    break;
                }
            }

            prefix = new;
        }

        prefix
    }

    // Autocomplete path
    fn complete_path(&self, target: &str) -> anyhow::Result<Vec<String>> {
        let path = Path::new(target);

        let (dir, base) = if target.ends_with('/') {
            (path, "")
        } else {
            (path.parent().unwrap_or(Path::new("")), path.file_name().unwrap().to_str().unwrap())
        };

        let dir = if dir.to_str().unwrap().is_empty() {
            "."
        } else {
            dir.to_str().unwrap()
        };

        let mut result = Vec::new();
           
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().display().to_string();

            if name.starts_with(base) {
                let mut full = format!("{}/{}", dir, name);

                if entry.file_type()?.is_dir() {
                    full.push('/');
                }

                result.push(full);
            }
        }

        Ok(result)
   
    }

}