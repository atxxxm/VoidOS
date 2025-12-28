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

        // Check if path exists
        if !path.exists() {
            eprintln!("rm: cannot remove '{}': No such file or directory", path_str);
            error_occurred = true;
            continue;
        }

        let res = if path.is_dir() {
            // Check if it's a directory
            if args.recursive {
                fs::remove_dir_all(path)
            } else {
                eprintln!("Cannot remove directory '{}': Is a directory", path_str);
                error_occurred = true;
                continue;
            }
        // Check if it's a file
        } else if path.is_file() {
            fs::remove_file(path)
        // Check if it's a symlink
        } else {
            fs::remove_dir(path)
        };

        // Handle result
        if let Err(e) = res {
            eprintln!("Failed to remove '{}': {}", path_str, e);
            error_occurred = true;
        }

        // Print success message
        if error_occurred {
            std::process::exit(1);
        }


    }
}
