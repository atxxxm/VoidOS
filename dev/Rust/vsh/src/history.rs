// Work with history

pub struct History {
    history_array: Vec<String>,
    history_index: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self { history_array: Vec::new(), history_index: None }
    }

    pub fn previous_command(&mut self) {
        todo!()
    }

    pub fn next_command(&mut self) {
        todo!()
    }

    pub fn add_command(&mut self, command: &str) {
        todo!()
    }

    pub fn get_command(&self) -> String {
        todo!()
    }

    pub fn zero_index(&mut self) {
        todo!()
    }
}