use std::fs;
use std::path::Path;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about = "Remove a directory", long_about = None)]
struct Args {
    /// Remove parent directories as well (-p)
    #[arg(short = 'p', long)]
    parents: bool,

    /// Directories to remove
    dirs: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // Check if there are any directories to remove
    if args.dirs.is_empty() {
        eprintln!("Missing operand");
        std::process::exit(1);
    }

    let mut error_occurred = false;

    for dir in &args.dirs {
        let mut path = Path::new(dir);

        // Check if directory exists
        if !path.exists() {
            eprintln!("Failed to remove directory '{}': No such directory", dir);
            error_occurred = true;
            continue;
        }

        // Check if it's a directory
        if !path.is_dir() {
            eprintln!("Failed to remove directory '{}': Not a directory", dir);
            error_occurred = true;
            continue;
        }

        if !args.parents {
            if let Err(e) = fs::remove_dir(path) {
                eprintln!("rmdir: failed to remove '{}': {}", path.display(), e);
                error_occurred = true;
            }
        } else {
            // Delete recursively upwards while directories are empty
            while path.exists() && path.is_dir() {
                match fs::remove_dir(path) {
                    Ok(_) => {
                        // Lets go upwards
                        if let Some(parent) = path.parent() {
                            path = parent;
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        
    }

    if error_occurred {
        std::process::exit(1);
    }
}