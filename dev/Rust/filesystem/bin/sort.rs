use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Sort lines of text", long_about = None)]
struct Args {
    /// Reverse the sort order
    #[arg(short = 'r', long)]
    reverse: bool,

    /// Sort numerically instead of lexicographically
    #[arg(short = 'n', long)]
    numeric: bool,

    /// Remove duplicate lines from the output
    #[arg(short = 'u', long)]
    unique: bool,

    /// Files to read (default: stdin)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let mut content = String::new();
    if args.paths.is_empty() {
        if io::stdin().read_to_string(&mut content).is_err() {
            return;
        }
    } else {
        for path in &args.paths {
            match fs::read_to_string(path) {
                Ok(text) => content.push_str(&text),
                Err(e) => eprintln!("sort: cannot open '{path}': {e}"),
            }
        }
    }

    let mut lines: Vec<&str> = content.lines().collect();
    sort_lines(&mut lines, args.numeric, args.reverse);

    if args.unique {
        // Duplicates are adjacent once sorted, so a plain dedup is enough.
        lines.dedup();
    }

    for line in lines {
        println!("{line}");
    }
}

fn sort_lines(lines: &mut [&str], numeric: bool, reverse: bool) {
    if numeric {
        lines.sort_by(|a, b| {
            let na: f64 = a.trim().parse().unwrap_or(0.0);
            let nb: f64 = b.trim().parse().unwrap_or(0.0);
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort_unstable();
    }
    if reverse {
        lines.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_lexicographically_by_default() {
        let mut lines = vec!["banana", "apple", "cherry"];
        sort_lines(&mut lines, false, false);
        assert_eq!(lines, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn sorts_numerically_when_requested() {
        let mut lines = vec!["10", "2", "1"];
        sort_lines(&mut lines, true, false);
        assert_eq!(lines, vec!["1", "2", "10"]);
    }

    #[test]
    fn reverses_the_order() {
        let mut lines = vec!["a", "b", "c"];
        sort_lines(&mut lines, false, true);
        assert_eq!(lines, vec!["c", "b", "a"]);
    }
}
