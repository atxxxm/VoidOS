use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Output the last part of files", long_about = None)]
struct Args {
    /// Number of lines to print
    #[arg(short = 'n', long, default_value_t = 10)]
    lines: usize,

    /// Files to read (default: stdin)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    if args.paths.is_empty() {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_ok() {
            print_tail(&input, args.lines);
        }
        return;
    }

    let more_than_one = args.paths.len() > 1;
    for (i, path) in args.paths.iter().enumerate() {
        match fs::read_to_string(path) {
            Ok(content) => {
                if more_than_one {
                    if i > 0 {
                        println!();
                    }
                    println!("==> {path} <==");
                }
                print_tail(&content, args.lines);
            }
            Err(e) => eprintln!("tail: cannot open '{path}': {e}"),
        }
    }
}

fn print_tail(content: &str, n: usize) {
    for line in tail_lines(content, n) {
        println!("{line}");
    }
}

fn tail_lines(content: &str, n: usize) -> Vec<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_last_n_lines() {
        assert_eq!(tail_lines("a\nb\nc\nd", 2), vec!["c", "d"]);
    }

    #[test]
    fn returns_everything_when_fewer_lines_than_requested() {
        assert_eq!(tail_lines("a\nb", 10), vec!["a", "b"]);
    }
}
