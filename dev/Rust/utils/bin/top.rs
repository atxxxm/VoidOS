use std::{
    collections::HashMap,
    fs,
    thread,
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(2);

fn main() {
    let clk_tck = clock_ticks_per_sec();
    let mut previous: HashMap<u32, u64> = HashMap::new();
    let mut previous_time = Instant::now();

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous_time).as_secs_f64().max(0.001);

        let mut rows = Vec::new();
        let mut current: HashMap<u32, u64> = HashMap::new();

        for pid in list_pids() {
            let Some(stat) = read_stat(pid) else { continue };
            current.insert(pid, stat.total_ticks);

            // Unseen pids (just started since the last refresh) show 0%
            // this round rather than a spurious spike from "all their
            // ticks so far, divided by one refresh interval".
            let prev_ticks = previous.get(&pid).copied().unwrap_or(stat.total_ticks);
            let delta_ticks = stat.total_ticks.saturating_sub(prev_ticks);
            let cpu_pct = (delta_ticks as f64 / clk_tck as f64) / elapsed * 100.0;

            let rss_kb = read_rss_kb(pid).unwrap_or(0);
            rows.push((pid, stat.state, cpu_pct, rss_kb, stat.comm));
        }

        rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        print!("\x1B[2J\x1B[H");
        println!("{:>6} {:<6} {:>6} {:>10} CMD", "PID", "STATE", "%CPU", "RSS(KB)");
        for (pid, state, cpu_pct, rss_kb, comm) in rows.iter().take(30) {
            println!("{pid:>6} {state:<6} {cpu_pct:>6.1} {rss_kb:>10} {comm}");
        }

        previous = current;
        previous_time = now;
        thread::sleep(REFRESH);
    }
}

struct Stat {
    comm: String,
    state: char,
    total_ticks: u64,
}

fn list_pids() -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut pids: Vec<u32> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    pids.sort_unstable();
    pids
}

fn read_stat(pid: u32) -> Option<Stat> {
    parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

// /proc/[pid]/stat's fields after the parenthesized comm: state is field
// 1 (0-indexed here), utime is field 11, stime is field 12 (fields 3, 14,
// and 15 of the full record, once you account for pid and comm).
fn parse_stat(text: &str) -> Option<Stat> {
    let comm_start = text.find('(')?;
    let comm_end = text.rfind(')')?;
    let comm = text[comm_start + 1..comm_end].to_string();
    let rest = text[comm_end + 1..].trim_start();
    let fields: Vec<&str> = rest.split_whitespace().collect();

    let state = fields.first()?.chars().next()?;
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(Stat { comm, state, total_ticks: utime + stime })
}

fn read_rss_kb(pid: u32) -> Option<u64> {
    parse_rss_kb(&fs::read_to_string(format!("/proc/{pid}/status")).ok()?)
}

fn parse_rss_kb(text: &str) -> Option<u64> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn clock_ticks_per_sec() -> i64 {
    #[cfg(unix)]
    {
        let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if ticks > 0 { ticks } else { 100 }
    }
    #[cfg(not(unix))]
    {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_after_a_parenthesized_comm() {
        // Fields after "pid (comm)": state, ppid, pgrp, session, tty,
        // tpgid, flags, minflt, cminflt, majflt, cmajflt, utime, stime.
        let stat = "42 (my proc) S 1 1 1 0 -1 0 0 0 0 0 150 50";
        let s = parse_stat(stat).unwrap();
        assert_eq!(s.comm, "my proc");
        assert_eq!(s.state, 'S');
        assert_eq!(s.total_ticks, 200);
    }

    #[test]
    fn parses_rss_from_status_text() {
        let status = "Name:\tbash\nVmRSS:\t  4096 kB\nThreads:\t1\n";
        assert_eq!(parse_rss_kb(status), Some(4096));
    }

    #[test]
    fn missing_rss_line_returns_none() {
        assert_eq!(parse_rss_kb("Name:\tbash\n"), None);
    }
}
