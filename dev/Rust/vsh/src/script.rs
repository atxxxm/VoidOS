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
        let mut lines = text.lines().peekable();

        while let Some(txt) = lines.next() {
            // Skips comments (this also covers the `#!/bin/vsh` shebang
            // line) and blank lines. Previously this also skipped any line
            // containing the substring "vsh" anywhere -- so a command like
            // `echo "welcome to vsh"` would silently vanish from the script.
            if txt.trim_start().starts_with('#') || txt.trim().is_empty() {
                continue;
            }

            // A function definition may open with "name() {" on its own
            // line and continue across several lines until one that's
            // just "}" -- collect the whole thing into one logical
            // command, joined the same way a single-line definition would
            // read, so it round-trips through the same `name() { body }`
            // parsing everywhere else in the shell.
            let trimmed = txt.trim_end();
            if trimmed.ends_with("() {") || trimmed.ends_with("(){") {
                let mut body_lines = Vec::new();
                for body_line in lines.by_ref() {
                    if body_line.trim() == "}" {
                        break;
                    }
                    body_lines.push(body_line.trim().to_string());
                }
                let header = trimmed.trim_end_matches('{').trim_end();
                let body = body_lines.join("; ");
                commands.push(format!("{header} {{ {body} }}"));
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

    #[test]
    fn collects_a_multiline_function_into_one_logical_command() {
        let path = std::env::temp_dir().join("vsh_script_test_multiline_function.vsh");
        {
            let mut file = std::fs::File::create(&path).unwrap();
            writeln!(file, "greet() {{").unwrap();
            writeln!(file, "  echo hi").unwrap();
            writeln!(file, "  echo there").unwrap();
            writeln!(file, "}}").unwrap();
            writeln!(file, "ls").unwrap();
        }

        let commands = Script::new(path.to_str().unwrap()).get_vsh_command().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            commands,
            vec![
                "greet() { echo hi; echo there }".to_string(),
                "ls".to_string(),
            ]
        );
    }
}