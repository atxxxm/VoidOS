use std::fs;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Create a directory", long_about = None)]
struct Args {
    /// Create intermediate directories if they do not exist
    #[arg(short = 'p', long)]
    parents: bool,

    /// Directory path to create
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // Check if paths are provided
    if args.paths.is_empty() {
        eprintln!("Missing operand");
        std::process::exit(1);
    }

    let mut error_occurred = false;

    // Create directories
    for path in &args.paths {   
        let res = if args.parents {
            fs::create_dir_all(path)
        } else {
            fs::create_dir(path)
        };

        if let Err(e) = res {
            eprintln!("Cannot create directory '{}': {}", path, e);
            error_occurred = true;
        }
    
    }

    // Exit with error code if any error occurred
    if error_occurred {
        std::process::exit(1);
    }
}