// Main functionality of the shell

use anyhow;
use crossterm::{event::{self, Event, KeyCode, KeyModifiers}, terminal};

use crate::{executor::Executor, history::History, prompt::Prompt, script::Script, utils::{get_current_username, is_script}};

pub struct Shell {
    last_exit: i32,
}

impl Shell {
    pub fn new() -> Self {
        unsafe { std::env::set_var("PATH", "/bin:/sbin"); }
        Self { last_exit: 0 }
    }

    // Run a command
    fn run_cmd(&mut self, command: &str, is_script: bool) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        let result = self.run_cmd_inner(command, is_script);
        terminal::enable_raw_mode()?;
        match result {
            Ok(code) => { self.last_exit = code; Ok(()) }
            Err(e) => Err(e),
        }
    }

    fn run_cmd_inner(&mut self, command: &str, is_script: bool) -> anyhow::Result<i32> {
        if is_script {
            let mut last = 0i32;
            for cmd in Script::new(command).get_vsh_command()? {
                if !cmd.trim().is_empty() {
                    last = Executor::new(&cmd, self.last_exit).run()?;
                    self.last_exit = last;
                }
            }
            Ok(last)
        } else {
            Ok(Executor::new(command, self.last_exit).run()?)
        }
    }

    // Main loop vsh
    pub fn run(&mut self) -> anyhow::Result<()> {
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
                        KeyCode::Char(ch) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            match ch {
                                'c' => {
                                    print!("^C\r\n");
                                    history.zero_index();
                                    prompt.default();
                                }
                                'd' if prompt.prompt.is_empty() => {
                                    terminal::disable_raw_mode()?;
                                    return Ok(());
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char(ch) => prompt.add(ch),
                        KeyCode::Backspace => prompt.delete(),
                        KeyCode::Home => prompt.cursor_start(),
                        KeyCode::End => prompt.cursor_end(),
                        KeyCode::Left => prompt.cursor_left(),
                        KeyCode::Right => prompt.cursor_right(),
                        KeyCode::Up => {
                            history.previous_command();
                            prompt.prompt = history.get_command();
                            prompt.cursor_end();
                        }
                        KeyCode::Down => {
                            history.next_command();
                            prompt.prompt = history.get_command();
                            prompt.cursor_end();
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