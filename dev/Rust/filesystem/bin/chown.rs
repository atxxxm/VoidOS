use clap::Parser;

#[derive(Parser)]
#[command(
    version,
    about = "Change file owner and group (numeric uid/gid only -- this OS has no /etc/passwd to resolve names)",
    long_about = None
)]
struct Args {
    /// "uid" or "uid:gid" (numeric only)
    owner: String,

    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let Some((uid, gid)) = parse_owner(&args.owner) else {
        eprintln!(
            "chown: invalid owner '{}': expected \"uid\" or \"uid:gid\" (numeric)",
            args.owner
        );
        std::process::exit(1);
    };

    if args.paths.is_empty() {
        eprintln!("chown: missing operand");
        std::process::exit(1);
    }

    let mut error_occurred = false;
    for path in &args.paths {
        if let Err(e) = set_owner(path, uid, gid) {
            eprintln!("chown: cannot access '{path}': {e}");
            error_occurred = true;
        }
    }

    if error_occurred {
        std::process::exit(1);
    }
}

// Returns (uid, gid); -1 for either means "leave unchanged", matching
// chown(2)'s own convention.
fn parse_owner(spec: &str) -> Option<(i64, i64)> {
    match spec.split_once(':') {
        Some((uid, gid)) => Some((uid.parse().ok()?, gid.parse().ok()?)),
        None => Some((spec.parse().ok()?, -1)),
    }
}

#[cfg(unix)]
fn set_owner(path: &str, uid: i64, gid: i64) -> std::io::Result<()> {
    use std::ffi::CString;
    let c_path = CString::new(path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { libc::chown(c_path.as_ptr(), uid as libc::uid_t, gid as libc::gid_t) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn set_owner(_path: &str, _uid: i64, _gid: i64) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "chown is only supported on Unix",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uid_only() {
        assert_eq!(parse_owner("1000"), Some((1000, -1)));
    }

    #[test]
    fn parses_uid_and_gid() {
        assert_eq!(parse_owner("1000:1000"), Some((1000, 1000)));
    }

    #[test]
    fn rejects_non_numeric_owners() {
        assert_eq!(parse_owner("alice"), None);
    }
}
