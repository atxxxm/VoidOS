use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Send a signal to a process", long_about = None)]
struct Args {
    /// Signal name (TERM, KILL, INT, HUP, ...) or number
    #[arg(short = 's', long, default_value = "TERM")]
    signal: String,

    /// Process ids to signal
    pids: Vec<i32>,
}

fn main() {
    let args = Args::parse();

    if args.pids.is_empty() {
        eprintln!("kill: missing operand");
        std::process::exit(1);
    }

    let Some(sig) = parse_signal(&args.signal) else {
        eprintln!("kill: invalid signal: '{}'", args.signal);
        std::process::exit(1);
    };

    let mut error_occurred = false;
    for pid in &args.pids {
        if let Err(e) = send_signal(*pid, sig) {
            eprintln!("kill: ({pid}): {e}");
            error_occurred = true;
        }
    }

    if error_occurred {
        std::process::exit(1);
    }
}

// Standard Linux signal numbers -- kept as plain integers (rather than
// referencing libc::SIG* constants here) so this lookup has no
// platform-specific compile requirements; only send_signal() below needs
// the Unix-only syscall.
fn parse_signal(s: &str) -> Option<i32> {
    let name = s.trim_start_matches("SIG").to_uppercase();
    Some(match name.as_str() {
        "HUP" => 1,
        "INT" => 2,
        "QUIT" => 3,
        "KILL" => 9,
        "USR1" => 10,
        "USR2" => 12,
        "TERM" => 15,
        "CONT" => 18,
        "STOP" => 19,
        _ => return s.parse::<i32>().ok(),
    })
}

#[cfg(unix)]
fn send_signal(pid: i32, sig: i32) -> std::io::Result<()> {
    let ret = unsafe { libc::kill(pid, sig) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn send_signal(_pid: i32, _sig: i32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "kill is only supported on Unix",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_names_with_or_without_the_sig_prefix() {
        assert_eq!(parse_signal("TERM"), Some(15));
        assert_eq!(parse_signal("SIGTERM"), Some(15));
        assert_eq!(parse_signal("kill"), Some(9));
    }

    #[test]
    fn falls_back_to_a_raw_number() {
        assert_eq!(parse_signal("9"), Some(9));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_signal("NOTASIGNAL"), None);
    }
}
