use anyhow;
use clap::Parser;
use colored::*;
use regex::RegexBuilder;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version, about = "Search for patterns in files", long_about = None)]
struct Args {
    /// Regex pattern to search for
    pattern: String,

    /// File or directory to search in (default: stdin)
    path: Option<String>,

    /// Ignore case distinctions
    #[arg(short = 'i', long)]
    ignore_case: bool,

    /// Search recursively in directories
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Count matches instead of showing them
    #[arg(short = 'c', long)]
    count: bool,

    /// Invert match (show lines that do NOT match)
    #[arg(short = 'v', long)]
    invert_match: bool,

    /// Show line numbers
    #[arg(short = 'n', long)]
    line_number: bool,

    /// Highlight matches with color
    #[arg(long)]
    color: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Create regex builder
    let re = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.ignore_case)
        .build()?;

    // Search files or stdin
    match &args.path {
        Some(path) => {
            let path = Path::new(path);
            if path.is_dir() {
                search_dir(path, &args, &re)?;
            } else {
                search_file(path, &args, &re)?;
            }
        }

        None => search_stdin(&args, &re)?,
    }

    Ok(())
}

fn search_dir(path: &Path, args: &Args, re: &regex::Regex) -> anyhow::Result<()> {
    // Recursive search
    if args.recursive {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() {
                search_file(path, args, re)?;
            }
        }
    } else {
        // Only one level
        for entry in fs::read_dir(path)? {
            let path = entry?.path();

            if path.is_file() {
                search_file(&path, args, re)?;
            }
        }
    }

    Ok(())
}

fn search_file(path: &Path, args: &Args, re: &regex::Regex) -> anyhow::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut count = 0;

    // Read line by line
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let is_match = re.is_match(&line);

        if is_match ^ args.invert_match {
            count += 1;
            if !args.count {
                print_line(path, i + 1, &line, re, args);
            }
        }
    }

    // Print count if needed and not counting lines
    if args.count {
        println!("{}:{}", path.display(), count);
    }

    Ok(())
}


// Search stdin
fn search_stdin(args: &Args, re: &regex::Regex) -> anyhow::Result<()> {
    let stdin = std::io::stdin();

    // Read line by line
    for (i, line) in stdin.lock().lines().enumerate() {
        let line = line?;
        let is_match = re.is_match(&line);

        if is_match ^ args.invert_match {
            if !args.count {
                print_line(Path::new("(stdin)"), i + 1, &line, re, args);
            }
        }
    }

    Ok(())
}

// Print line
fn print_line(path: &Path, line_num: usize, line: &str, re: &regex::Regex, args: &Args) {
    let mut out_line = line.to_string();

    // Colorize matches
    if args.color {
        out_line = re
            .replace_all(line, |caps: &regex::Captures| caps[0].red().bold().to_string())
            .to_string();
    }

    // Line number
    if args.line_number {
        println!("{}:{}: {}", path.display(), line_num, out_line);
    } else {
        println!("{}: {}", path.display(), out_line);
    }
}