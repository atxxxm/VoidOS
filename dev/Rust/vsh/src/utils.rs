
use std::{fs::File, io::{Read, BufReader, BufRead}, path::Path};
use anyhow;

const USERSPACE_PATH: &str = "/etc/userspace.toml";

// Get current username
pub fn get_current_username() -> anyhow::Result<String> {
    if !Path::new(USERSPACE_PATH).exists() {
        return Ok("void".to_string());
    }

    let mut file = File::open(USERSPACE_PATH)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;

    for txt in text.lines() {
        if txt.contains("current") {
            let current_name = txt.split('=').nth(1).unwrap().trim();
            let name = current_name.replace('"', "");
            return Ok(name);
        }
    }

    Ok("void".to_string())
}

// Split command and arguments
pub fn split_cmd_and_args(cmd: &str) -> (String, Vec<String>) {
    let mut parts = cmd.split_whitespace();
    let cmd = parts.next().unwrap_or("").to_string();
    let args = parts.map(|s| s.to_string()).collect();
    (cmd, args)
}

// Expand variables, ~ and special params in a single word.
// Single-quoted sections are passed through literally (no expansion).
// Double-quoted sections are expanded but treated as one word.
// Both quote types are stripped from the result.
pub fn expand(s: &str, last_exit: i32) -> String {
    // Tilde expansion: ~ or ~/... at the start of the word
    let s: std::borrow::Cow<str> = if s == "~" || s.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        format!("{}{}", home, &s[1..]).into()
    } else {
        s.into()
    };

    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => { in_single = !in_single; }
            '"'  if !in_single => { in_double = !in_double; }
            '$'  if !in_single => match chars.peek().copied() {
                Some('?') => { chars.next(); result.push_str(&last_exit.to_string()); }
                Some('$') => { chars.next(); result.push_str(&std::process::id().to_string()); }
                Some('{') => {
                    chars.next();
                    let mut name = String::new();
                    for c in chars.by_ref() {
                        if c == '}' { break; }
                        name.push(c);
                    }
                    result.push_str(&std::env::var(&name).unwrap_or_default());
                }
                Some(c) if c.is_alphabetic() || c == '_' => {
                    let mut name = String::new();
                    while let Some(&c) = chars.peek() {
                        if !c.is_alphanumeric() && c != '_' { break; }
                        name.push(c);
                        chars.next();
                    }
                    result.push_str(&std::env::var(&name).unwrap_or_default());
                }
                _ => result.push('$'),
            },
            _ => result.push(ch),
        }
    }

    result
}

// Get current path
pub fn get_current_path() -> String {
    let mut current_path = String::new();

    if let Ok(cur_path) = std::env::current_dir() {
        current_path = cur_path.display().to_string();
    }

    current_path
}

// Check if a file is a script
pub fn is_script(path: &str) -> anyhow::Result<bool> {
    if !Path::new(path).exists() {
        return Ok(false);
    }

    let lines = read_first_lines(path, 5)?;

    for line in lines {
        if line.starts_with("#!") && line.contains("vsh") {
            return Ok(true);
        }
    }

    Ok(false)
}

// Reading a specified number of initial lines in a file
fn read_first_lines(filename: &str, n: usize) -> std::io::Result<Vec<String>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    reader.lines().take(n).collect::<Result<Vec<_>, _>>()
}


// Enum for mechanisms
#[derive(PartialEq, Eq, Clone)]
pub enum Mechanisms {
    Pipe,              // |
    LogicalOr,         // ||
    LogicalAnd,        // &&
    RedirectionOut,    // >
    RedirectionAppend, // >>
    RedirectionIn,     // <
    ConsistentExec,    // ;
    RedirectionErrOut,    // 2>
    RedirectionErrAppend, // 2>>
    RedirectionErrToOut,  // 2>&1
    None,
}

// Token struct
#[derive(Clone)]
pub struct Token {
    pub content: String,
    pub mechanism: Mechanisms,
}

pub struct Tokenize {
    prompt: String,
}

impl Tokenize {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
        }
    }

    // Tokenize command
    pub fn tokenize(&self) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        let ch: Vec<char> = self.prompt.chars().collect();
        let mut i: usize = 0;
        let len = ch.len();

        while i < len {
            let c = ch[i];

            if c == ' ' {
                i += 1;
                continue;
            }

            if i + 1 < len {
                let two = format!("{}{}", ch[i], ch[i + 1]);

                if two == "||" {
                    tokens.push(Token { content: "||".to_string(), mechanism: Mechanisms::LogicalOr });
                    i += 2;
                    continue;
                }

                if two == "&&" {
                    tokens.push(Token { content: "&&".to_string(), mechanism: Mechanisms::LogicalAnd });
                    i += 2;
                    continue;
                }

                if two == ">>" {
                    tokens.push(Token { content: ">>".to_string(), mechanism: Mechanisms::RedirectionAppend });
                    i += 2;
                    continue;
                }
                
            }

            match c {
                '|' => {
                    tokens.push(Token { content: "|".to_string(), mechanism: Mechanisms::Pipe });
                    i += 1;
                    continue;
                }
                '>' => {
                    tokens.push(Token { content: ">".to_string(), mechanism: Mechanisms::RedirectionOut });
                    i += 1;
                    continue;
                }
                '<' => {
                    tokens.push(Token { content: "<".to_string(), mechanism: Mechanisms::RedirectionIn });
                    i += 1;
                    continue;
                }
                ';' => {
                    tokens.push(Token { content: ";".to_string(), mechanism: Mechanisms::ConsistentExec });
                    i += 1;
                    continue;
                }

                _ => {}
            }

            // Fd redirect: 2>, 2>>, 2>&1
            if c == '2' && i + 1 < len && ch[i + 1] == '>' {
                if i + 3 < len && ch[i + 2] == '&' && ch[i + 3] == '1' {
                    tokens.push(Token { content: "2>&1".into(), mechanism: Mechanisms::RedirectionErrToOut });
                    i += 4;
                } else if i + 2 < len && ch[i + 2] == '>' {
                    tokens.push(Token { content: "2>>".into(), mechanism: Mechanisms::RedirectionErrAppend });
                    i += 3;
                } else {
                    tokens.push(Token { content: "2>".into(), mechanism: Mechanisms::RedirectionErrOut });
                    i += 2;
                }
                continue;
            }

            let start = i;

            while i < len {
                let wc = ch[i];
                if "|&><;".contains(wc) { break; }
                // Stop before a fd redirect so it becomes its own token
                if wc == '2' && i + 1 < len && ch[i + 1] == '>' { break; }
                if wc == '"' || wc == '\'' {
                    let quote = wc;
                    i += 1;
                    while i < len && ch[i] != quote {
                        i += 1;
                    }
                }
                i += 1;
            }

            let content: String = ch[start..i].iter().collect();
            tokens.push(Token { content: content.trim().to_string(), mechanism: Mechanisms::None });

        }

        tokens
    }
}

// Exec tree
pub enum ExecNode {
    Command {
        program: String,
        args: Vec<String>,
        stdin: Option<String>,
        stdout: Option<String>,
        append: bool,
        stderr: Option<String>,
        err_append: bool,
        stderr_to_stdout: bool,
    },

    Pipe {
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },

    LogicalAnd {
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },

    LogicalOr {
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },

    Sequence {
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    }
}

// Token parse
pub struct TokenParse {
    tokens: Vec<Token>,
}

impl TokenParse {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    // Parsing by priority hierarchy //

    // Parsing
    pub fn parse(&self) -> ExecNode {
        self.parse_sequence(&self.tokens)
    }

    // Parse ;
    fn parse_sequence(&self, tokens: &[Token]) -> ExecNode {
        if let Some(pos) = self.find_last_mech(tokens, Mechanisms::ConsistentExec) {
            return ExecNode::Sequence {
                left: Box::new(self.parse_sequence(&tokens[..pos])),
                right: Box::new(self.parse_sequence(&tokens[pos + 1..])),
            };
        }

        self.parse_logical(tokens)
    }

    // Parse || and &&
    fn parse_logical(&self, tokens: &[Token]) -> ExecNode {
        if let Some(pos) = self.find_last_mech(tokens, Mechanisms::LogicalAnd) {
            return ExecNode::LogicalAnd {
                left: Box::new(self.parse_logical(&tokens[..pos])),
                right: Box::new(self.parse_logical(&tokens[pos + 1..])),
            };
        }

        if let Some(pos) = self.find_last_mech(tokens, Mechanisms::LogicalOr) {
            return ExecNode::LogicalOr {
                left: Box::new(self.parse_logical(&tokens[..pos])),
                right: Box::new(self.parse_logical(&tokens[pos + 1..])),
            };
        }

        self.parse_pipe(tokens)
    }



    // Parse |
    fn parse_pipe(&self, tokens: &[Token]) -> ExecNode {
        if let Some(pos) = self.find_last_mech(tokens, Mechanisms::Pipe) {
            return ExecNode::Pipe {
                left: Box::new(self.parse_pipe(&tokens[..pos])),
                right: Box::new(self.parse_pipe(&tokens[pos + 1..])),
            };
        }

        self.parse_command(tokens)
    }

    // Collect comnands + redirects
    fn parse_command(&self, tokens: &[Token]) -> ExecNode {
        let mut program = String::new();
        let mut args = Vec::new();
        let mut stdin = None;
        let mut stdout = None;
        let mut append = false;
        let mut stderr = None;
        let mut err_append = false;
        let mut stderr_to_stdout = false;

        let mut i = 0;

        while i < tokens.len() {
            let t = &tokens[i];

            match t.mechanism {
                Mechanisms::RedirectionIn => {
                    stdin = Some(tokens[i + 1].content.clone());
                    i += 2;
                }
                Mechanisms::RedirectionOut => {
                    stdout = Some(tokens[i + 1].content.clone());
                    append = false;
                    i += 2;
                }
                Mechanisms::RedirectionAppend => {
                    stdout = Some(tokens[i + 1].content.clone());
                    append = true;
                    i += 2;
                }
                Mechanisms::RedirectionErrOut => {
                    stderr = Some(tokens[i + 1].content.clone());
                    err_append = false;
                    i += 2;
                }
                Mechanisms::RedirectionErrAppend => {
                    stderr = Some(tokens[i + 1].content.clone());
                    err_append = true;
                    i += 2;
                }
                Mechanisms::RedirectionErrToOut => {
                    stderr_to_stdout = true;
                    i += 1;
                }
                Mechanisms::None => {
                    if program.is_empty() {
                        let (cmd, a) = split_cmd_and_args(&t.content);
                        program = cmd;
                        args.extend(a);
                    } else {
                        args.push(t.content.clone());
                    }
                    i += 1;
                }

                _ => panic!("Unexpected operator in parse_command"),
            }
        }

        ExecNode::Command {
            program,
            args,
            stdin,
            stdout,
            append,
            stderr,
            err_append,
            stderr_to_stdout,
        }
    }

    // Low priority operator search
    fn find_last_mech(&self, tokens: &[Token], mech: Mechanisms) -> Option<usize> {
        for (i, t) in tokens.iter().enumerate().rev() {
            if t.mechanism == mech {
                return Some(i);
            }
        }

        None
    }
}


