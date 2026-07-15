use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Print newline, word, and byte counts", long_about = None)]
struct Args {
    /// Print the newline count
    #[arg(short = 'l', long)]
    lines: bool,

    /// Print the word count
    #[arg(short = 'w', long)]
    words: bool,

    /// Print the byte count
    #[arg(short = 'c', long)]
    bytes: bool,

    /// Files to read (default: stdin)
    paths: Vec<String>,
}

struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
}

fn count(content: &str) -> Counts {
    Counts {
        lines: content.lines().count(),
        words: content.split_whitespace().count(),
        bytes: content.len(),
    }
}

fn print_counts(c: &Counts, args: &Args, label: Option<&str>) {
    let show_all = !args.lines && !args.words && !args.bytes;
    let mut parts = Vec::new();
    if args.lines || show_all {
        parts.push(format!("{:>7}", c.lines));
    }
    if args.words || show_all {
        parts.push(format!("{:>7}", c.words));
    }
    if args.bytes || show_all {
        parts.push(format!("{:>7}", c.bytes));
    }
    let line = parts.join(" ");
    match label {
        Some(l) => println!("{line} {l}"),
        None => println!("{line}"),
    }
}

fn main() {
    let args = Args::parse();

    if args.paths.is_empty() {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_ok() {
            print_counts(&count(&input), &args, None);
        }
        return;
    }

    let mut total = Counts { lines: 0, words: 0, bytes: 0 };
    let mut error_occurred = false;
    for path in &args.paths {
        match fs::read_to_string(path) {
            Ok(content) => {
                let c = count(&content);
                total.lines += c.lines;
                total.words += c.words;
                total.bytes += c.bytes;
                print_counts(&c, &args, Some(path));
            }
            Err(e) => {
                eprintln!("wc: cannot open '{path}': {e}");
                error_occurred = true;
            }
        }
    }
    if args.paths.len() > 1 {
        print_counts(&total, &args, Some("total"));
    }

    if error_occurred {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_lines_words_and_bytes() {
        let c = count("hello world\nfoo\n");
        assert_eq!(c.lines, 2);
        assert_eq!(c.words, 3);
        assert_eq!(c.bytes, 16);
    }

    #[test]
    fn empty_input_counts_as_zero() {
        let c = count("");
        assert_eq!((c.lines, c.words, c.bytes), (0, 0, 0));
    }
}
