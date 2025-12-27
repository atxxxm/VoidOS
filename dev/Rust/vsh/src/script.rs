// Work with scripts

use std::{fs::File, io::Read};

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
    
    // Get commands from script
    pub fn get_vsh_command(&self) -> anyhow::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;

        let mut commands: Vec<String> = Vec::new();

        for txt in text.lines() {
            if txt.contains("vsh") || txt.trim_start().starts_with('#') 
                || txt.trim().is_empty() {
                    continue;
                }

            commands.push(txt.to_string());
        }

        Ok(commands)
    }
}