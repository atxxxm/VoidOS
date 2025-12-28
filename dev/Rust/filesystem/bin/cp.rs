use std::fs;
use std::path::Path;
use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version, about = "Copy files and directories", long_about = None)]
struct Args {
    /// Recursively copy
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Source path
    from: String,

    /// Destination path
    to: String,
}

fn main() {
    let args = Args::parse();

    let from = Path::new(&args.from);
    let to = Path::new(&args.to);

    if !from.exists() {
        eprintln!("File not found: {}", from.display());
        std::process::exit(1);
    }

    // If source - file
    if from.is_file() {
        if let Err(e) = copy_file(from, to) {
            eprintln!("cp: {} -> {}: {}", from.display(), to.display(), e);
        }

        return;
    }

    // If source - directory
    if from.is_dir() {
        if !args.recursive {
            eprintln!("cp: {} is a directory", from.display());
            std::process::exit(1);
        }

        if let Err(e) = copy_dir_recursive(from, to) {
            eprintln!("Failed to copy directory: {}", e);
            std::process::exit(1);
        }

        return;
    }

    eprintln!("Unknown file type: {}", from.display());
    std::process::exit(1);
}


// Copy file
fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let dest = if to.is_dir() {
        to.join(from.file_name().unwrap())
    } else {
        to.to_path_buf()
    };

    fs::copy(from, dest)?;

    Ok(())
}

// Copy directory
fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(from).unwrap();
        let dest = to.join(relative_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(path, dest)?;
        }
    }

    Ok(())
}