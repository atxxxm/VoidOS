use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Remove sections from each line of input", long_about = None)]
struct Args {
    /// Delimiter (default: tab)
    #[arg(short = 'd', long, default_value = "\t")]
    delimiter: String,

    /// Fields to extract, e.g. "1,3" or "2-4" (1-indexed)
    #[arg(short = 'f', long)]
    fields: Option<String>,

    /// Character positions to extract, e.g. "1-5" (1-indexed)
    #[arg(short = 'c', long)]
    chars: Option<String>,

    /// Files to read (default: stdin)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let spec = match (&args.fields, &args.chars) {
        (Some(f), _) => f,
        (None, Some(c)) => c,
        (None, None) => {
            eprintln!("cut: you must specify -f or -c");
            std::process::exit(1);
        }
    };
    let ranges = parse_ranges(spec);

    let process = |content: &str| {
        for line in content.lines() {
            let cut = if args.chars.is_some() {
                cut_chars(line, &ranges)
            } else {
                cut_fields(line, &args.delimiter, &ranges)
            };
            println!("{cut}");
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
            Err(e) => eprintln!("cut: cannot open '{path}': {e}"),
        }
    }
}

// Parses "1,3-5,8" into a sorted, deduplicated list of 1-indexed positions.
fn parse_ranges(spec: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    for part in spec.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let start: usize = start.parse().unwrap_or(1);
            let end: usize = end.parse().unwrap_or(start);
            positions.extend(start..=end);
        } else if let Ok(n) = part.parse::<usize>() {
            positions.push(n);
        }
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

fn cut_fields(line: &str, delimiter: &str, positions: &[usize]) -> String {
    let fields: Vec<&str> = line.split(delimiter).collect();
    positions
        .iter()
        .filter_map(|&pos| fields.get(pos.wrapping_sub(1)).copied())
        .collect::<Vec<_>>()
        .join(delimiter)
}

fn cut_chars(line: &str, positions: &[usize]) -> String {
    let chars: Vec<char> = line.chars().collect();
    positions
        .iter()
        .filter_map(|&pos| chars.get(pos.wrapping_sub(1)).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_ranges_and_singles() {
        assert_eq!(parse_ranges("1,3-5,8"), vec![1, 3, 4, 5, 8]);
    }

    #[test]
    fn cuts_fields_by_delimiter() {
        assert_eq!(cut_fields("a:b:c:d", ":", &[1, 3]), "a:c");
    }

    #[test]
    fn cuts_character_ranges() {
        assert_eq!(cut_chars("hello", &[1, 2, 3]), "hel");
    }
}
