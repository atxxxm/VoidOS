use std::fs;
use std::path::Path;
use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version, about = "Move files and directories", long_about = None)]
struct Args {
    /// Recursive move (-r)
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
        eprintln!("Cannot stat '{}': No such file or directory", args.from);
    }

    // If it's file try simply renaming it
    if from.is_file() {
        if let Err(_) = fs::rename(from, to) {
            // If rename failed, try copying and removing
            if let Err(e) = fs::copy(from, to) {
                eprintln!("mv: {} -> {}: {}", from.display(), to.display(), e);
                std::process::exit(1);
            }

            if let Err(e) = fs::remove_file(from) {
                eprintln!(
                    "mv: {} -> {}: {}",
                    from.display(),
                    to.display(),
                    e
                );
                std::process::exit(1);
            }
        }

        return;
    }


    // If it's directory
    if from.is_dir() {
        if !args.recursive {
            eprintln!("Failed to move directory: {}", from.display());
            std::process::exit(1);
        }

        // Try renaming the directory
        if let Err(_) = fs::rename(from, to) {
            // If renaming failed, try copying and removing
            if let Err(e) = copy_dir_recursive(from, to) {
                eprintln!("Failed to move directory: {}", e);
                std::process::exit(1);
            }

            if let Err(e) = fs::remove_dir_all(from) {
                eprintln!(
                    "Failed to move directory: {}",
                    e
                );
                std::process::exit(1);
            }
        }
    }

}

// Copy directory recursively
fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    // Create destination directory if it doesn't exist
    for entry in WalkDir::new(from) {
        let entry = entry?;
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(from).unwrap();
        let dest_path = to.join(rel_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy file
            fs::copy(src_path, dest_path)?;
        }
    }

    Ok(())
}