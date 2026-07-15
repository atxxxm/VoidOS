use std::fs;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Output the first part of files", long_about = None)]
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
            print_head(&input, args.lines);
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
                print_head(&content, args.lines);
            }
            Err(e) => eprintln!("head: cannot open '{path}': {e}"),
        }
    }
}

fn print_head(content: &str, n: usize) {
    for line in content.lines().take(n) {
        println!("{line}");
    }
}
