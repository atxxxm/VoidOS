use std::path::Path;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Output file contents", long_about = None)]
struct Args {
    /// Output from the end of the file
    #[arg(short = 'r', long)]
    reverse: bool,

    /// The path to the file
    path: String,
}

fn main() {
    let args = Args::parse();

    let path = Path::new(&args.path);

    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        std::process::exit(1);
    }

    if !path.is_file() {
        eprintln!("Not a file: {}", path.display());
        std::process::exit(1);
    }

    if args.reverse {
        let contents = std::fs::read_to_string(&args.path).expect("File reading error!");

        let lines: Vec<&str> = contents.lines().collect();
        let reversed_lines: Vec<&str> = lines.into_iter().rev().collect();

        let mut text = String::new();

        for line in reversed_lines {
            text.push_str(&format!("{line}\n"));
        }

        println!("{text}");
    } else {
        let contents = std::fs::read_to_string(&args.path).expect("File reading error!");

        println!("{}", contents);
    }
}
