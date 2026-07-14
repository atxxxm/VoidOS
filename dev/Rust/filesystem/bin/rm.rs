use std::fs;
use std::path::Path;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Remove files and directories", long_about = None)]
struct Args {
    /// Recursively remove directories and their contents
    #[arg(short, long)]
    recursive: bool,

    /// Paths to remove
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

    // Process each path
    for path_str in &args.paths {
        let path = Path::new(path_str);

        // Use symlink_metadata so broken/dangling symlinks are detected
        // instead of being silently followed (and reported as missing).
        let meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => {
                eprintln!("rm: cannot remove '{}': No such file or directory", path_str);
                error_occurred = true;
                continue;
            }
        };

        let res = if meta.is_symlink() {
            fs::remove_file(path)
        } else if meta.is_dir() {
            if args.recursive {
                fs::remove_dir_all(path)
            } else {
                eprintln!("Cannot remove directory '{}': Is a directory", path_str);
                error_occurred = true;
                continue;
            }
        } else {
            fs::remove_file(path)
        };

        // Handle result
        if let Err(e) = res {
            eprintln!("Failed to remove '{}': {}", path_str, e);
            error_occurred = true;
        }
    }

    if error_occurred {
        std::process::exit(1);
    }
}
