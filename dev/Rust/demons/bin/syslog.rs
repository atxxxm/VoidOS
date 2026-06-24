use nix::sys::signal::{self, SigHandler, Signal};
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    io::{self, Write},
    os::unix::net::UnixDatagram,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const SOCKET_PATH: &str = "/dev/log";
const LOG_FILE: &str = "/var/log/messages";

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

fn log_to_file(msg: &str) {
    let timestamp = chrono::Local::now().format("%b %d %H:%M:%S");
    if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(LOG_FILE) {
        writeln!(f, "{timestamp} {msg}").ok();
    }
}

fn main() {
    unsafe {
        signal::signal(Signal::SIGTERM, SigHandler::Handler(handle_signal)).unwrap();
        signal::signal(Signal::SIGINT, SigHandler::Handler(handle_signal)).unwrap();
    }

    fs::create_dir_all("/var/log").ok();
    fs::remove_file(SOCKET_PATH).ok();

    let sock = UnixDatagram::bind(SOCKET_PATH).expect("syslogd: failed to bind socket");
    std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o666)).ok();
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();

    println!("syslogd started, listening on {SOCKET_PATH}");

    let mut buf = [0u8; 1024];

    while RUNNING.load(Ordering::Relaxed) {
        match sock.recv(&mut buf) {
            Ok(len) if len > 0 => {
                let msg = std::str::from_utf8(&buf[..len])
                    .unwrap_or("[invalid utf8]")
                    .trim_end_matches('\n');
                log_to_file(msg);
            }
            Ok(_) => {}
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut
                    || e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => eprintln!("syslogd: recv error: {e}"),
        }
    }

    drop(sock);
    fs::remove_file(SOCKET_PATH).ok();
    println!("syslogd stopped");
}
