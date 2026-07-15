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
            // Skips comments (this also covers the `#!/bin/vsh` shebang
            // line) and blank lines. Previously this also skipped any line
            // containing the substring "vsh" anywhere -- so a command like
            // `echo "welcome to vsh"` would silently vanish from the script.
            if txt.trim_start().starts_with('#') || txt.trim().is_empty() {
                continue;
            }

            commands.push(txt.to_string());
        }

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn keeps_command_lines_that_merely_mention_vsh() {
        let path = std::env::temp_dir().join("vsh_script_test_keeps_vsh_mentions.vsh");
        {
            let mut file = std::fs::File::create(&path).unwrap();
            writeln!(file, "#!/bin/vsh").unwrap();
            writeln!(file, "echo \"welcome to vsh\"").unwrap();
            writeln!(file, "# a comment").unwrap();
            writeln!(file).unwrap();
            writeln!(file, "ls").unwrap();
        }

        let commands = Script::new(path.to_str().unwrap()).get_vsh_command().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            commands,
            vec!["echo \"welcome to vsh\"".to_string(), "ls".to_string()]
        );
    }
}