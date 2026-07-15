use std::path::{Path, PathBuf};
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Create links between files", long_about = None)]
struct Args {
    /// Create a symbolic link instead of a hard link
    #[arg(short = 's', long)]
    symbolic: bool,

    target: String,
    link_name: String,
}

fn main() {
    let args = Args::parse();

    let link_name = target_link_path(&args.target, &args.link_name);

    let result = if args.symbolic {
        make_symlink(&args.target, &link_name)
    } else {
        std::fs::hard_link(&args.target, &link_name)
    };

    if let Err(e) = result {
        eprintln!(
            "ln: cannot link '{}' to '{}': {e}",
            args.target,
            link_name.display()
        );
        std::process::exit(1);
    }
}

// If link_name is an existing directory, the link is created inside it
// under the target's own filename (matching coreutils' `ln`).
fn target_link_path(target: &str, link_name: &str) -> PathBuf {
    let link_path = Path::new(link_name);
    if link_path.is_dir() {
        let file_name = Path::new(target).file_name().unwrap_or_default();
        link_path.join(file_name)
    } else {
        link_path.to_path_buf()
    }
}

#[cfg(unix)]
fn make_symlink(target: &str, link_name: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link_name)
}

#[cfg(not(unix))]
fn make_symlink(_target: &str, _link_name: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symbolic links are only supported on Unix",
    ))
}
