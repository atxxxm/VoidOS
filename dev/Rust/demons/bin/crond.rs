use chrono::{Datelike, Timelike};
use nix::sys::signal::{self, SigHandler, Signal};
use std::{
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
    fs,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

struct Job {
    min: String,
    hour: String,
    mday: String,
    mon: String,
    wday: String,
    cmd: String,
}

fn matches_field(value: u32, field: &str) -> bool {
    if field == "*" {
        return true;
    }
    field.parse::<u32>().map_or(false, |v| v == value)
}

fn parse_crontab(path: &str) -> Vec<Job> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("crond: cannot open {path}: {e}");
            std::process::exit(1);
        }
    };

    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let min  = parts.next()?.to_string();
            let hour = parts.next()?.to_string();
            let mday = parts.next()?.to_string();
            let mon  = parts.next()?.to_string();
            let wday = parts.next()?.to_string();
            let cmd  = parts.collect::<Vec<_>>().join(" ");
            if cmd.is_empty() { return None; }
            Some(Job { min, hour, mday, mon, wday, cmd })
        })
        .collect()
}

fn run_job(cmd: &str) {
    let mut parts = cmd.split_whitespace();
    let Some(prog) = parts.next() else { return };
    let args: Vec<&str> = parts.collect();

    match Command::new(prog).args(&args).spawn() {
        Ok(_) => println!("crond: running {cmd}"),
        Err(e) => eprintln!("crond: failed to run {cmd}: {e}"),
    }
}

fn main() {
    unsafe {
        signal::signal(Signal::SIGTERM, SigHandler::Handler(handle_signal)).unwrap();
        signal::signal(Signal::SIGINT, SigHandler::Handler(handle_signal)).unwrap();
    }

    println!("crond started");

    let jobs = parse_crontab("/etc/crontab");
    let mut last_minute = u32::MAX;

    while RUNNING.load(Ordering::Relaxed) {
        let now = chrono::Local::now();
        let min = now.minute();

        if min != last_minute {
            last_minute = min;

            for job in &jobs {
                if matches_field(now.minute(), &job.min)
                    && matches_field(now.hour(), &job.hour)
                    && matches_field(now.day(), &job.mday)
                    && matches_field(now.month(), &job.mon)
                    && matches_field(now.weekday().num_days_from_sunday(), &job.wday)
                {
                    run_job(&job.cmd);
                }
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("crond stopped");
}
