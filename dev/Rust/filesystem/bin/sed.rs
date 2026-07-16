use std::fs;
use std::io::{self, Read};
use clap::Parser;
use regex::Regex;

#[derive(Parser)]
#[command(
    version,
    about = "Stream editor -- only supports the 's/pattern/replacement/flags' script form",
    long_about = None
)]
struct Args {
    /// Editing script, e.g. 's/foo/bar/g'
    script: String,

    /// Files to read (default: stdin)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let Some(substitution) = parse_substitution(&args.script) else {
        eprintln!("sed: unsupported script (only 's/pattern/replacement/flags' is supported)");
        std::process::exit(1);
    };

    let re = match Regex::new(&substitution.pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("sed: invalid pattern: {e}");
            std::process::exit(1);
        }
    };

    let process = |content: &str| {
        for line in content.lines() {
            println!("{}", apply_substitution(&re, &substitution, line));
        }
    };

    if args.paths.is_empty() {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_ok() {
            process(&input);
        }
        return;
    }

    for path in &args.paths {
        match fs::read_to_string(path) {
            Ok(content) => process(&content),
            Err(e) => eprintln!("sed: cannot open '{path}': {e}"),
        }
    }
}

struct Substitution {
    pattern: String,
    replacement: String,
    global: bool,
}

// Parses `s/pattern/replacement/flags`. The delimiter is always '/'
// (unlike real sed, which lets you pick another character) -- a literal
// '/' inside the pattern or replacement must be escaped as `\/`.
fn parse_substitution(script: &str) -> Option<Substitution> {
    let rest = script.strip_prefix("s/")?;
    let parts = split_unescaped(rest, '/');
    if parts.len() != 3 {
        return None;
    }
    let pattern = parts[0].replace("\\/", "/");
    let replacement = parts[1].replace("\\/", "/");
    let global = parts[2].contains('g');
    Some(Substitution { pattern, replacement, global })
}

fn split_unescaped(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&delim) {
            current.push(delim);
            chars.next();
        } else if c == delim {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
}

fn apply_substitution(re: &Regex, sub: &Substitution, line: &str) -> String {
    // regex's own replacement syntax already understands $1/$name, so the
    // parsed "replacement" text can be passed through directly.
    if sub.global {
        re.replace_all(line, sub.replacement.as_str()).into_owned()
    } else {
        re.replace(line, sub.replacement.as_str()).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_substitution() {
        let s = parse_substitution("s/foo/bar/g").unwrap();
        assert_eq!(s.pattern, "foo");
        assert_eq!(s.replacement, "bar");
        assert!(s.global);
    }

    #[test]
    fn parses_an_escaped_delimiter() {
        let s = parse_substitution("s/a\\/b/c/").unwrap();
        assert_eq!(s.pattern, "a/b");
        assert_eq!(s.replacement, "c");
        assert!(!s.global);
    }

    #[test]
    fn rejects_scripts_that_are_not_substitutions() {
        assert!(parse_substitution("d").is_none());
    }

    #[test]
    fn replaces_only_the_first_match_without_g() {
        let s = parse_substitution("s/a/X/").unwrap();
        let re = Regex::new(&s.pattern).unwrap();
        assert_eq!(apply_substitution(&re, &s, "a a a"), "X a a");
    }

    #[test]
    fn replaces_all_matches_with_g() {
        let s = parse_substitution("s/a/X/g").unwrap();
        let re = Regex::new(&s.pattern).unwrap();
        assert_eq!(apply_substitution(&re, &s, "a a a"), "X X X");
    }
}
