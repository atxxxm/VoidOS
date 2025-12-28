// Main functionality of the shell

use anyhow;
use crossterm::{event::{self, Event, KeyCode}, terminal};

use crate::{executor::Executor, history::History, prompt::Prompt, script::Script, utils::{get_current_username, is_script}};

pub struct Shell;

impl Shell {
    pub fn new() -> Self {
        // set PATH
        unsafe { std::env::set_var("PATH", "/bin:/sbin"); }

        Self
    }

    // Run a command
    fn run_cmd(&self, command: &str, is_script: bool) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        if is_script {
            let commands = Script::new(command).get_vsh_command()?;

            for cmd in commands {
                if cmd.trim().is_empty() {
                    continue;
                }

                let mut exec = Executor::new(&cmd);
                exec.run()?;
            }

            return Ok(());
        }

        let mut exec = Executor::new(command);
        exec.run()?;

        terminal::enable_raw_mode()?;

        Ok(())
    }

    // Main loop vsh
    pub fn run(&self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        let username = get_current_username()?;
        let stdout = std::io::stdout();
        let mut prompt = Prompt::new(username, stdout, '|');

        let mut history = History::new();

        loop {
            prompt.default();
            prompt.print()?;

            loop {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char(ch) => prompt.add(ch),
                        KeyCode::Backspace => prompt.delete(),
                        KeyCode::Home => prompt.cursor_start(),
                        KeyCode::End => prompt.cursor_end(),
                        KeyCode::Left => prompt.cursor_left(),
                        KeyCode::Right => prompt.cursor_right(),
                        KeyCode::Up => {
                            history.previous_command();
                            prompt.prompt = history.get_command();
                            prompt.cursor_pos = prompt.prompt.len();
                        }
                        KeyCode::Down => {
                            history.next_command();
                            prompt.prompt = history.get_command();
                            prompt.cursor_pos = prompt.prompt.len();
                        }
                        KeyCode::Tab => prompt.autocomplete()?,
                        KeyCode::Enter => {
                            print!("\r\n");
                            history.add_command(&prompt.prompt);
                            history.zero_index();
                            let is_script = is_script(&prompt.prompt)?;
                            self.run_cmd(&prompt.prompt, is_script)?;
                            break;
                        }
                        KeyCode::Esc => {
                            terminal::disable_raw_mode()?;
                            return Ok(());
                        }

                        _ => {
                            continue;
                        }
                    }
                }

                prompt.print()?;
            }
        }
    }
}