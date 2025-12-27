// Work with scripts


use anyhow;
pub struct Script {
    path: String,
}

impl Script {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
    
    pub fn get_vsh_command(&self) -> anyhow::Result<Vec<String>> {
        todo!()
    }
}