
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
    let mut words = split_words(cmd).into_iter();
    let cmd = words.next().unwrap_or_default();
    let args = words.collect();
    (cmd, args)
}

// Split into whitespace-separated words without splitting inside quotes.
// Quote characters are left in place so `expand()` can strip/interpret
// them per word afterwards.
fn split_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Outside single quotes, a backslash escapes the next
            // character -- keep both here (expand() strips the backslash
            // later) so it can't be mistaken for a quote-toggle or a
            // word-splitting space in the meantime.
            '\\' if !in_single => {
                current.push(c);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(c);
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

// Expand variables, ~, backslash escapes, and command substitution in a
// single word. Single-quoted sections are passed through completely
// literally (no expansion, no escapes). Double-quoted sections are
// expanded but treated as one word. Both quote types are stripped from
// the result.
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
            // Outside quotes, `\` always escapes the next character
            // literally. Inside double quotes, only `\"`, `\\`, and `\$`
            // are recognized escapes (matching bash); any other backslash
            // is kept as-is. Single quotes suppress this arm entirely --
            // nothing is escapable there.
            '\\' if !in_single => match chars.peek().copied() {
                Some(next) if in_double => {
                    if next == '"' || next == '\\' || next == '$' {
                        chars.next();
                        result.push(next);
                    } else {
                        result.push('\\');
                    }
                }
                Some(next) => {
                    chars.next();
                    result.push(next);
                }
                None => result.push('\\'),
            },
            '\'' if !in_double => { in_single = !in_single; }
            '"'  if !in_single => { in_double = !in_double; }
            '$'  if !in_single => match chars.peek().copied() {
                Some('?') => { chars.next(); result.push_str(&last_exit.to_string()); }
                Some('$') => { chars.next(); result.push_str(&std::process::id().to_string()); }
                Some('(') => {
                    chars.next();
                    let inner = take_balanced_parens(&mut chars);
                    result.push_str(&run_captured(&inner));
                }
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
            '`' if !in_single => {
                let mut inner = String::new();
                for c in chars.by_ref() {
                    if c == '`' { break; }
                    inner.push(c);
                }
                result.push_str(&run_captured(&inner));
            }
            _ => result.push(ch),
        }
    }

    result
}

// Expands a single argument token into one or more final strings: normal
// expand() first, then (only for words that had no quoting at all) shell
// globbing against the filesystem. Skipping globbing for anything that was
// quoted/escaped keeps "was this `*` a literal or a wildcard" unambiguous
// without needing to track quotedness per-character all the way through
// expand()'s output -- the common case (`ls *.txt`, fully unquoted) is what
// this is for; a pattern with any quoting in it (`"pre"*.txt`) is left
// alone rather than risk mis-expanding an intentionally literal character.
pub fn expand_arg(raw: &str, last_exit: i32) -> Vec<String> {
    let expanded = expand(raw, last_exit);
    let was_quoted = raw.contains('\'') || raw.contains('"');
    if was_quoted || !(expanded.contains('*') || expanded.contains('?')) {
        return vec![expanded];
    }

    let matches = glob_match(&expanded);
    if matches.is_empty() {
        // No match: bash's default (non-nullglob) behavior is to pass the
        // pattern through literally rather than dropping the argument.
        vec![expanded]
    } else {
        matches
    }
}

fn glob_match(pattern: &str) -> Vec<String> {
    let (dir_prefix, search_dir, name_pattern) = match pattern.rfind('/') {
        Some(idx) => {
            let dir = &pattern[..idx];
            let dir = if dir.is_empty() { "/" } else { dir };
            (format!("{dir}/"), dir.to_string(), &pattern[idx + 1..])
        }
        None => (String::new(), ".".to_string(), pattern),
    };

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return Vec::new();
    };

    let mut matches: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            // Hidden files only match a pattern that itself starts with
            // '.', matching the usual shell convention.
            if name.starts_with('.') && !name_pattern.starts_with('.') {
                return false;
            }
            glob_name_match(name_pattern, name)
        })
        .map(|name| format!("{dir_prefix}{name}"))
        .collect();

    matches.sort();
    matches
}

// Matches a filename against a pattern using only `*` and `?` wildcards
// (no `[...]` bracket classes -- not needed for the common `*.ext` /
// `file?.log` cases this is meant to cover). Standard backtracking glob
// match.
fn glob_name_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_rec(&p, &n)
}

fn glob_match_rec(p: &[char], n: &[char]) -> bool {
    match p.first() {
        None => n.is_empty(),
        Some('*') => glob_match_rec(&p[1..], n) || (!n.is_empty() && glob_match_rec(p, &n[1..])),
        Some('?') => !n.is_empty() && glob_match_rec(&p[1..], &n[1..]),
        Some(c) => !n.is_empty() && n[0] == *c && glob_match_rec(&p[1..], &n[1..]),
    }
}

// Consumes up to (and including) the closing `)` matching the one that was
// already opened, tracking nesting depth so `$(echo $(date))` works.
// Deliberately not quote-aware inside the substitution itself -- a `)`
// inside a quoted string within the substituted command is a rare enough
// case that the added complexity isn't worth it here.
fn take_balanced_parens(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut depth = 1;
    let mut inner = String::new();
    for c in chars.by_ref() {
        match c {
            '(' => {
                depth += 1;
                inner.push(c);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                inner.push(c);
            }
            _ => inner.push(c),
        }
    }
    inner
}

// Command substitution: re-invokes this same vsh binary as `vsh -c
// <command>` and captures its stdout, rather than threading a callback
// through expand()'s call sites (which live in executor.rs, itself built
// on top of this module -- a callback would be circular). This also gets
// the substituted command vsh's full parser/executor (pipes, redirects,
// everything) for free, at the cost of one process fork per substitution.
// $? inside the substitution starts fresh at 0 rather than inheriting the
// caller's, and variable assignments made inside it don't leak back out
// -- both match real subshell semantics.
fn run_captured(command: &str) -> String {
    let exe = std::env::current_exe().unwrap_or_else(|_| "/bin/vsh".into());
    match std::process::Command::new(exe).arg("-c").arg(command).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
        Err(_) => String::new(),
    }
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
    Background,        // &
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
                // A lone '&' (not part of "&&", already handled above, or
                // "2>&1", handled below and never reached as a standalone
                // '&' since that whole sequence is consumed in one step).
                '&' => {
                    tokens.push(Token { content: "&".to_string(), mechanism: Mechanisms::Background });
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
#[derive(Clone)]
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
    },

    // A job to run without waiting for it, e.g. `sleep 5 &`. Only ever
    // produced at the top-level `;`/`&` list layer, never nested inside a
    // pipe/logical/command -- so a backgrounded sub-tree can never itself
    // contain another Background node.
    Background(Box<ExecNode>),
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
        self.parse_list(&self.tokens)
    }

    // Parse `;` and `&` -- the lowest-precedence, top-level job separators.
    // Unlike the other operators here (which nest on the *last* match,
    // building a right-heavy tree), this splits on the *first* separator
    // and recurses into the remainder. That matters for `&`: in
    // `a & b & c`, backgrounding must apply to each segment individually
    // (job1=a, job2=b, then c runs in the foreground) rather than to
    // whatever was built up to its left -- a last-match split would instead
    // wrap the entire "a & b" sequence in one Background node.
    fn parse_list(&self, tokens: &[Token]) -> ExecNode {
        let split = tokens.iter().enumerate().find(|(_, t)| {
            matches!(t.mechanism, Mechanisms::ConsistentExec | Mechanisms::Background)
        });

        let Some((pos, token)) = split else {
            return self.parse_logical(tokens);
        };

        let mut left = self.parse_logical(&tokens[..pos]);
        if token.mechanism == Mechanisms::Background {
            left = ExecNode::Background(Box::new(left));
        }

        let rest = &tokens[pos + 1..];
        if rest.is_empty() {
            return left;
        }

        ExecNode::Sequence {
            left: Box::new(left),
            right: Box::new(self.parse_list(rest)),
        }
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_word_stays_one_arg() {
        let (cmd, args) = split_cmd_and_args("echo \"hello world\" foo");
        assert_eq!(cmd, "echo");
        assert_eq!(args, vec!["\"hello world\"", "foo"]);
    }

    #[test]
    fn expand_strips_quotes_after_split() {
        let (_, args) = split_cmd_and_args("echo \"hello world\"");
        assert_eq!(expand(&args[0], 0), "hello world");
    }

    fn program_name(node: &ExecNode) -> &str {
        match node {
            ExecNode::Command { program, .. } => program.as_str(),
            _ => panic!("expected a Command node"),
        }
    }

    fn parse(input: &str) -> ExecNode {
        TokenParse::new(Tokenize::new(input).tokenize()).parse()
    }

    #[test]
    fn trailing_ampersand_backgrounds_the_command() {
        let root = parse("sleep 5 &");
        let ExecNode::Background(inner) = &root else {
            panic!("expected a Background node");
        };
        assert_eq!(program_name(inner), "sleep");
    }

    #[test]
    fn each_segment_in_a_chain_of_background_jobs_backgrounds_individually() {
        // Regression guard for the leftmost-split parsing: a naive
        // rightmost split (matching the other operators here) would wrap
        // the whole "a & b" in one Background node instead of
        // backgrounding "a" and "b" separately with "c" left foreground.
        let root = parse("a & b & c");

        let ExecNode::Sequence { left, right } = &root else {
            panic!("expected a Sequence node");
        };
        let ExecNode::Background(a) = left.as_ref() else {
            panic!("expected the first segment to be backgrounded");
        };
        assert_eq!(program_name(a), "a");

        let ExecNode::Sequence { left, right } = right.as_ref() else {
            panic!("expected a nested Sequence node");
        };
        let ExecNode::Background(b) = left.as_ref() else {
            panic!("expected the second segment to be backgrounded");
        };
        assert_eq!(program_name(b), "b");
        assert_eq!(program_name(right), "c");
    }

    #[test]
    fn semicolon_chains_still_run_in_written_order() {
        let root = parse("a ; b ; c");
        let ExecNode::Sequence { left, right } = &root else {
            panic!("expected a Sequence node");
        };
        assert_eq!(program_name(left), "a");

        let ExecNode::Sequence { left, right } = right.as_ref() else {
            panic!("expected a nested Sequence node");
        };
        assert_eq!(program_name(left), "b");
        assert_eq!(program_name(right), "c");
    }

    // -- backslash escaping -------------------------------------------------

    #[test]
    fn unquoted_backslash_escapes_dollar_and_space_literally() {
        assert_eq!(expand("\\$HOME", 0), "$HOME");
        assert_eq!(expand("a\\ b", 0), "a b");
    }

    #[test]
    fn double_quotes_only_recognize_a_few_escapes() {
        // \$ is a recognized double-quote escape...
        assert_eq!(expand("\"\\$HOME\"", 0), "$HOME");
        // ...but \d isn't, so the backslash survives.
        assert_eq!(expand("\"\\d\"", 0), "\\d");
    }

    #[test]
    fn single_quotes_never_escape() {
        assert_eq!(expand("'\\$HOME'", 0), "\\$HOME");
    }

    #[test]
    fn escaped_pipe_does_not_break_the_word_apart() {
        let (cmd, args) = split_cmd_and_args("echo \\| foo");
        assert_eq!(cmd, "echo");
        assert_eq!(args, vec!["\\|", "foo"]);
        assert_eq!(expand(&args[0], 0), "|");
    }

    // -- globbing -------------------------------------------------------------

    #[test]
    fn glob_name_match_handles_star_and_question_mark() {
        assert!(glob_name_match("*.txt", "a.txt"));
        assert!(!glob_name_match("*.txt", "a.txtx"));
        assert!(!glob_name_match("*.txt", "b.log"));
        assert!(glob_name_match("file?.log", "file1.log"));
        assert!(!glob_name_match("file?.log", "file12.log"));
    }

    #[test]
    fn expand_arg_leaves_quoted_patterns_untouched() {
        // A quoted `*` should never be treated as a wildcard.
        assert_eq!(expand_arg("\"*.txt\"", 0), vec!["*.txt".to_string()]);
    }

    #[test]
    fn expand_arg_globs_unquoted_patterns_against_the_filesystem() {
        // Built with an explicit forward slash rather than Path::display()
        // (which uses '\' on Windows, where these unit tests run natively
        // -- vsh's expand() correctly treats '\' as a Unix shell escape
        // character, which would eat a Windows-style separator; the
        // pattern itself just needs to be a valid path string, however
        // it's spelled).
        let dir = std::env::temp_dir()
            .join("vsh_glob_test")
            .to_string_lossy()
            .replace('\\', "/");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(format!("{dir}/a.txt"), "").unwrap();
        std::fs::write(format!("{dir}/b.txt"), "").unwrap();
        std::fs::write(format!("{dir}/c.log"), "").unwrap();

        let pattern = format!("{dir}/*.txt");
        let mut matches = expand_arg(&pattern, 0);
        matches.sort();

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(matches, vec![format!("{dir}/a.txt"), format!("{dir}/b.txt")]);
    }

    #[test]
    fn expand_arg_passes_through_a_pattern_with_no_matches() {
        assert_eq!(
            expand_arg("/definitely/not/a/real/dir/*.nope", 0),
            vec!["/definitely/not/a/real/dir/*.nope".to_string()]
        );
    }

    // -- command substitution parsing (not actually running a subshell) -----

    #[test]
    fn take_balanced_parens_handles_nesting() {
        // "$(" has already been consumed by the caller (expand()) by the
        // time this runs -- simulate that by feeding it everything after.
        let mut chars = "echo $(date))rest".chars().peekable();
        let inner = take_balanced_parens(&mut chars);
        assert_eq!(inner, "echo $(date)");
        assert_eq!(chars.collect::<String>(), "rest");
    }
}
