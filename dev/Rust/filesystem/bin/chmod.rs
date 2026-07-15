use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Change file mode bits", long_about = None)]
struct Args {
    /// Octal mode, e.g. 755
    mode: String,

    /// Files to change
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let Ok(mode) = u32::from_str_radix(&args.mode, 8) else {
        eprintln!("chmod: invalid mode: '{}'", args.mode);
        std::process::exit(1);
    };

    if args.paths.is_empty() {
        eprintln!("chmod: missing operand");
        std::process::exit(1);
    }

    let mut error_occurred = false;
    for path in &args.paths {
        if let Err(e) = set_mode(path, mode) {
            eprintln!("chmod: cannot access '{path}': {e}");
            error_occurred = true;
        }
    }

    if error_occurred {
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn set_mode(path: &str, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &str, _mode: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "chmod is only supported on Unix",
    ))
}
