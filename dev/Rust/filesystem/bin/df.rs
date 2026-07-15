use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Report filesystem disk space usage", long_about = None)]
struct Args {
    /// Paths to report on (default: /)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let paths = if args.paths.is_empty() {
        vec!["/".to_string()]
    } else {
        args.paths
    };

    println!(
        "{:>10} {:>10} {:>10} {:>5}  Mounted on",
        "Size", "Used", "Avail", "Use%"
    );

    let mut error_occurred = false;
    for path in &paths {
        match disk_usage(path) {
            Ok((total, avail)) => {
                let used = total.saturating_sub(avail);
                let pct = used.checked_mul(100).and_then(|n| n.checked_div(total)).unwrap_or(0);
                println!(
                    "{:>10} {:>10} {:>10} {:>4}%  {}",
                    human(total),
                    human(used),
                    human(avail),
                    pct,
                    path
                );
            }
            Err(e) => {
                eprintln!("df: cannot read '{path}': {e}");
                error_occurred = true;
            }
        }
    }

    if error_occurred {
        std::process::exit(1);
    }
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1}{}", UNITS[unit])
}

#[cfg(unix)]
fn disk_usage(path: &str) -> std::io::Result<(u64, u64)> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let stat = unsafe { stat.assume_init() };
    let total = stat.f_blocks as u64 * stat.f_frsize as u64;
    let avail = stat.f_bavail as u64 * stat.f_frsize as u64;
    Ok((total, avail))
}

#[cfg(not(unix))]
fn disk_usage(_path: &str) -> std::io::Result<(u64, u64)> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "df is only supported on Unix",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_human_readable_sizes() {
        assert_eq!(human(500), "500.0B");
        assert_eq!(human(2048), "2.0K");
        assert_eq!(human(5 * 1024 * 1024), "5.0M");
    }
}
