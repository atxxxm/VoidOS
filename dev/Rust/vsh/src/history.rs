// Work with history

pub struct History {
    history_array: Vec<String>,
    history_index: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self { history_array: Vec::new(), history_index: None }
    }

    // Previous command
    pub fn previous_command(&mut self) {
        if self.history_array.is_empty() {
            return;
        }

        if let Some(idx) = self.history_index {
            if idx > 0 {
                self.history_index = Some(idx - 1);
            }
        } else {
            self.history_index = Some(self.history_array.len() - 1);
        }
    }

    // Next command
    pub fn next_command(&mut self) {
        if self.history_array.is_empty() {
            return;
        }

        if let Some(idx) = self.history_index {
            if idx < self.history_array.len() - 1 {
                self.history_index = Some(idx + 1);
            } else {
                self.history_index = None;
            }
        }
    }

    // Add command to history
    pub fn add_command(&mut self, command: &str) {
        self.history_array.push(command.to_string());
    }

    // Get command from history
    pub fn get_command(&self) -> String {
        if let Some(idx) = self.history_index {
            return self.history_array[idx].clone();
        }

        return String::new();
    }

    pub fn zero_index(&mut self) {
        self.history_index = None;
    }
}