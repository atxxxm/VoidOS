// Main functionality of the shell

use std::collections::HashMap;

use anyhow;
use crossterm::{event::{self, Event, KeyCode, KeyModifiers}, terminal};

use crate::{executor::{Executor, ignore_sigint}, history::History, jobs::JobTable, prompt::Prompt, script::Script, utils::{get_current_username, is_script}};


pub struct Shell {
    last_exit: i32,
    history: History,
    jobs: JobTable,
    aliases: HashMap<String, String>,
    functions: HashMap<String, String>,
}

impl Shell {
    pub fn new() -> Self {
        unsafe { std::env::set_var("PATH", "/bin:/sbin"); }
        // See executor::ignore_sigint for why: without this, Ctrl+C while
        // a foreground command is running would kill vsh along with it.
        ignore_sigint();
        Self {
            last_exit: 0,
            history: History::new(),
            jobs: JobTable::new(),
            aliases: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    // Non-interactive entry point (`vsh -c "command"`): no raw mode, no
    // prompt loop, just run one line and report its exit code. Used both
    // for a user-facing `-c` flag and, recursively, by expand()'s command
    // substitution ($()/backticks), which re-invokes this same binary.
    pub fn run_single(&mut self, command: &str) -> anyhow::Result<i32> {
        self.run_cmd_inner(command, false)
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

    // Runs one Executor to completion and absorbs any `&` jobs it spawned
    // into the shell's (persistent, cross-command) job table.
    fn run_one(&mut self, cmd: &str) -> anyhow::Result<i32> {
        let mut executor = Executor::new(cmd, self.last_exit);
        let code = executor.run()?;
        self.jobs.absorb(executor.take_background_jobs());
        Ok(code)
    }

    // Handles function definitions/calls, then (for interactive input
    // only -- scripts don't alias-expand by default, matching bash)
    // alias expansion, before finally executing through the real
    // Executor. Shared by both the top-level typed line and each line
    // read from a script, so functions defined in either place are
    // callable from either place too.
    fn run_line(&mut self, line: &str, allow_aliases: bool) -> anyhow::Result<i32> {
        let trimmed = line.trim();

        if let Some((name, body)) = parse_function_def(trimmed) {
            self.functions.insert(name, body);
            return Ok(0);
        }

        let line = if allow_aliases { self.expand_aliases(line) } else { line.to_string() };

        let first_word = line.split_whitespace().next().unwrap_or("");
        if let Some(body) = self.functions.get(first_word).cloned() {
            // Functions run as a fixed body with no positional parameter
            // ($1, $2, ...) support yet -- any words typed after the
            // function's name are currently just ignored.
            return self.run_one(&body);
        }

        self.run_one(&line)
    }

    // Single-pass, depth-limited alias expansion of the first word only
    // (so `alias ll='ls -la'` then typing `ll -a` works). Real bash also
    // expands aliases in other command positions of a compound line
    // (`foo && ll`) -- that's not covered here, kept simple on purpose.
    fn expand_aliases(&self, line: &str) -> String {
        let mut current = line.to_string();
        for _ in 0..10 {
            let trimmed = current.trim_start();
            let first_word_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            let first_word = &trimmed[..first_word_end];
            let Some(value) = self.aliases.get(first_word) else {
                break;
            };
            let rest = &trimmed[first_word_end..];
            current = format!("{value}{rest}");
        }
        current
    }

    fn handle_alias(&mut self, rest: &str) -> anyhow::Result<i32> {
        if rest.is_empty() {
            for (name, value) in &self.aliases {
                println!("alias {name}='{value}'");
            }
            return Ok(0);
        }

        if let Some((name, value)) = rest.split_once('=') {
            let name = name.trim();
            let value = value.trim().trim_matches('\'').trim_matches('"');
            self.aliases.insert(name.to_string(), value.to_string());
            return Ok(0);
        }

        match self.aliases.get(rest) {
            Some(value) => {
                println!("alias {rest}='{value}'");
                Ok(0)
            }
            None => {
                eprintln!("alias: {rest}: not found");
                Ok(1)
            }
        }
    }

    // `!!` (last command) / `!N` (command N from history) expansion.
    // Applied before the expanded text is added to history and run, so
    // (like bash) the *expanded* command ends up in history, not the
    // literal "!!"/"!N".
    fn expand_history_refs(&self, line: &str) -> String {
        let trimmed = line.trim();
        if trimmed == "!!" {
            return self.history.all().last().cloned().unwrap_or_default();
        }
        if let Some(rest) = trimmed.strip_prefix('!')
            && let Ok(n) = rest.parse::<usize>()
            && let Some(cmd) = self.history.all().get(n.saturating_sub(1))
        {
            return cmd.clone();
        }
        line.to_string()
    }

    // Most recent history entry containing `query`, skipping the `skip`
    // nearest matches (used to cycle to older matches on repeated Ctrl+R).
    fn search_history(&self, query: &str, skip: usize) -> Option<String> {
        if query.is_empty() {
            return None;
        }
        self.history.all().iter().rev().filter(|cmd| cmd.contains(query)).nth(skip).cloned()
    }

    fn run_cmd_inner(&mut self, command: &str, is_script: bool) -> anyhow::Result<i32> {
        let trimmed = command.trim();

        // history builtin: print numbered history list
        if trimmed == "history" {
            for (i, entry) in self.history.all().iter().enumerate() {
                println!("{:>4}  {}", i + 1, entry);
            }
            return Ok(0);
        }

        if trimmed == "jobs" {
            for line in self.jobs.list_and_purge_done() {
                println!("{line}");
            }
            return Ok(0);
        }

        if let Some(rest) = trimmed.strip_prefix("fg") {
            let rest = rest.trim();
            if rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit() || c == '%') {
                let id = parse_job_id(rest).or_else(|| self.jobs.most_recent_id());
                return Ok(match id.and_then(|id| self.jobs.bring_to_foreground(id)) {
                    Some(code) => code,
                    None => {
                        eprintln!("fg: no such job");
                        1
                    }
                });
            }
        }

        if let Some(rest) = trimmed.strip_prefix("bg") {
            let rest = rest.trim();
            if rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit() || c == '%') {
                let id = parse_job_id(rest).or_else(|| self.jobs.most_recent_id());
                return Ok(match id {
                    Some(id) if self.jobs.contains(id) => {
                        println!("[{id}] already running in background");
                        0
                    }
                    _ => {
                        eprintln!("bg: no such job");
                        1
                    }
                });
            }
        }

        if let Some(rest) = trimmed.strip_prefix("alias") {
            return self.handle_alias(rest.trim());
        }
        if let Some(name) = trimmed.strip_prefix("unalias ") {
            self.aliases.remove(name.trim());
            return Ok(0);
        }

        if is_script {
            let mut last = 0i32;
            for cmd in Script::new(command).get_vsh_command()? {
                if !cmd.trim().is_empty() {
                    last = self.run_line(&cmd, false)?;
                    self.last_exit = last;
                }
            }
            Ok(last)
        } else {
            self.run_line(command, true)
        }
    }

    // Main loop vsh
    pub fn run(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        let username = get_current_username()?;
        let stdout = std::io::stdout();
        let mut prompt = Prompt::new(username, stdout, '|');

        // Ctrl+R reverse history search: (query, how many matches to skip,
        // to cycle to older ones on repeated Ctrl+R). `saved_prompt` is
        // what to restore the prompt to if the search is cancelled.
        let mut search: Option<(String, usize)> = None;
        let mut saved_prompt = String::new();

        loop {
            self.jobs.announce_finished();
            prompt.default();
            prompt.print()?;

            loop {
                if let Event::Key(key) = event::read()? {
                    if search.is_some() {
                        let mut exit_search = false;
                        let mut command_to_run: Option<String> = None;

                        if let Some((query, skip)) = &mut search {
                            match key.code {
                                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    *skip += 1;
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    exit_search = true;
                                }
                                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    query.push(ch);
                                    *skip = 0;
                                }
                                KeyCode::Backspace => {
                                    query.pop();
                                    *skip = 0;
                                }
                                KeyCode::Enter => {
                                    exit_search = true;
                                    command_to_run = self.search_history(query, *skip);
                                }
                                KeyCode::Esc => {
                                    exit_search = true;
                                }
                                _ => {}
                            }
                        }

                        if exit_search {
                            match command_to_run {
                                Some(cmd) => {
                                    print!("\r\n");
                                    let expanded = self.expand_history_refs(&cmd);
                                    self.history.add_command(&expanded);
                                    self.history.zero_index();
                                    let is_script = is_script(&expanded)?;
                                    search = None;
                                    self.run_cmd(&expanded, is_script)?;
                                    break;
                                }
                                None => {
                                    prompt.prompt = saved_prompt.clone();
                                    prompt.cursor_end();
                                    search = None;
                                }
                            }
                        } else if let Some((query, skip)) = &search {
                            let found = self.search_history(query, *skip);
                            prompt.prompt = match &found {
                                Some(cmd) => format!("(reverse-search)`{query}': {cmd}"),
                                None => format!("(failed reverse-search)`{query}': "),
                            };
                            prompt.cursor_end();
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                saved_prompt = prompt.prompt.clone();
                                search = Some((String::new(), 0));
                                prompt.prompt = "(reverse-search)`': ".to_string();
                                prompt.cursor_end();
                            }
                            KeyCode::Char(ch) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                match ch {
                                    'c' => {
                                        print!("^C\r\n");
                                        self.history.zero_index();
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
                                self.history.previous_command();
                                prompt.prompt = self.history.get_command();
                                prompt.cursor_end();
                            }
                            KeyCode::Down => {
                                self.history.next_command();
                                prompt.prompt = self.history.get_command();
                                prompt.cursor_end();
                            }
                            KeyCode::Tab => prompt.autocomplete()?,
                            KeyCode::Enter => {
                                print!("\r\n");
                                let expanded = self.expand_history_refs(&prompt.prompt);
                                self.history.add_command(&expanded);
                                self.history.zero_index();
                                let is_script = is_script(&expanded)?;
                                self.run_cmd(&expanded, is_script)?;
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
                }

                prompt.print()?;
            }
        }
    }
}

// Accepts bash-style job references: "%1", "1", or "" (meaning "the most
// recently started job", resolved by the caller).
fn parse_job_id(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    s.trim_start_matches('%').parse::<usize>().ok()
}

// Recognizes a (single-line) function definition: `name() { body }`.
// Multi-line definitions in scripts are collapsed to this same shape by
// Script::get_vsh_command before reaching here.
fn parse_function_def(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let paren_start = trimmed.find("()")?;
    let name = trimmed[..paren_start].trim();

    let first = name.chars().next()?;
    if !(first.is_alphabetic() || first == '_') {
        return None;
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    let after_parens = trimmed[paren_start + 2..].trim_start();
    let body = after_parens.strip_prefix('{')?;
    let body = body.strip_suffix('}')?;
    Some((name.to_string(), body.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_single_line_function_definition() {
        let (name, body) = parse_function_def("greet() { echo hi; }").unwrap();
        assert_eq!(name, "greet");
        assert_eq!(body, "echo hi;");
    }

    #[test]
    fn rejects_names_with_invalid_characters() {
        assert!(parse_function_def("1abc() { echo hi }").is_none());
        assert!(parse_function_def("ab-c() { echo hi }").is_none());
    }

    #[test]
    fn rejects_lines_that_are_not_function_definitions() {
        assert!(parse_function_def("echo hello").is_none());
        assert!(parse_function_def("ls ()").is_none());
    }
}
