use nix::{
    sys::signal::{self, kill, SigHandler, Signal},
    unistd::Pid,
};
use std::{
    process::{Child, Command},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
    fs,
};

const DEMONS_FILE: &str = "/etc/demons.d";
const DEMONS_DIR: &str = "/etc/demons/";

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_sigterm(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

struct Demon {
    name: String,
    path: String,
    child: Option<Child>,
}

impl Demon {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: format!("{DEMONS_DIR}{name}"),
            child: None,
        }
    }

    fn start(&mut self) {
        match Command::new(&self.path).spawn() {
            Ok(child) => {
                println!("Started demon: {} (pid={})", self.name, child.id());
                self.child = Some(child);
            }
            Err(e) => eprintln!("[{}] failed to start: {e}", self.name),
        }
    }

    fn check_and_restart(&mut self) {
        let Some(ref mut child) = self.child else { return };

        match child.try_wait() {
            Ok(Some(status)) => {
                self.child = None;
                if status.success() {
                    println!("[{}] exited normally, not restarting", self.name);
                } else {
                    println!("[{}] exited (status={status:?}), restarting...", self.name);
                    thread::sleep(Duration::from_secs(1));
                    self.start();
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("[{}] try_wait error: {e}", self.name),
        }
    }

    fn terminate(&mut self) {
        if let Some(ref child) = self.child {
            let pid = Pid::from_raw(child.id() as i32);
            kill(pid, Signal::SIGTERM).ok();
        }
        if let Some(mut child) = self.child.take() {
            child.wait().ok();
        }
    }
}

fn main() {
    unsafe {
        signal::signal(Signal::SIGTERM, SigHandler::Handler(handle_sigterm)).unwrap();
    }

    let content = match fs::read_to_string(DEMONS_FILE) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot open {DEMONS_FILE}: {e}");
            std::process::exit(1);
        }
    };

    let mut demons: Vec<Demon> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(Demon::new)
        .collect();

    for demon in &mut demons {
        demon.start();
    }

    println!("Demon Controller: entering main loop");

    while RUNNING.load(Ordering::Relaxed) {
        for demon in &mut demons {
            demon.check_and_restart();
        }
        thread::sleep(Duration::from_secs(1));
    }

    println!("Demon Controller: shutting down...");
    for demon in &mut demons {
        demon.terminate();
    }
}
