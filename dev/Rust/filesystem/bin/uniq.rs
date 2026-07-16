use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Report or omit repeated lines", long_about = None)]
struct Args {
    /// Prefix each line with its occurrence count
    #[arg(short = 'c', long)]
    count: bool,

    /// Only print lines that were duplicated
    #[arg(short = 'd', long)]
    duplicates_only: bool,

    /// Only print lines that were NOT duplicated
    #[arg(short = 'u', long)]
    unique_only: bool,

    /// File to read (default: stdin)
    path: Option<String>,
}

fn main() {
    let args = Args::parse();

    let content = match &args.path {
        Some(path) => match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("uniq: cannot open '{path}': {e}");
                std::process::exit(1);
            }
        },
        None => {
            let mut input = String::new();
            if io::stdin().read_to_string(&mut input).is_err() {
                return;
            }
            input
        }
    };

    for (line, count) in collapse_runs(content.lines()) {
        if args.duplicates_only && count < 2 {
            continue;
        }
        if args.unique_only && count > 1 {
            continue;
        }
        if args.count {
            println!("{count:>7} {line}");
        } else {
            println!("{line}");
        }
    }
}

// Collapses consecutive equal lines into (line, run-length) pairs -- like
// real `uniq`, this only merges *adjacent* duplicates; pipe through `sort`
// first if you want it applied across the whole input.
fn collapse_runs<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<(&'a str, usize)> {
    let mut out: Vec<(&str, usize)> = Vec::new();
    for line in lines {
        match out.last_mut() {
            Some((prev, count)) if *prev == line => *count += 1,
            _ => out.push((line, 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_only_adjacent_duplicates() {
        let lines = vec!["a", "a", "b", "a", "a", "a"];
        assert_eq!(
            collapse_runs(lines.into_iter()),
            vec![("a", 2), ("b", 1), ("a", 3)]
        );
    }
}
