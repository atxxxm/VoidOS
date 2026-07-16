use std::fs;

use colored::{Color, Colorize};
use serde::Deserialize;

// ASCII-only art and layout on purpose: over a serial console whose
// codepage isn't UTF-8, Unicode box-drawing/block characters come out as
// mojibake (learned the hard way with bit's editor). Plain '@' and spaces
// survive any encoding.
const LOGO: [&str; 10] = [
    "    @@@@@@@@@@@@@@@@@@@@@   ",
    "   @@@@@  @@@@@@@@  @@@@@@   ",
    "  @@@@@@  @@@@@@@@  @@@@@@@  ",
    " @@@@@@@@  @@@@@@  @@@@@@@@@ ",
    " @@@@@@@@@  @@@@  @@@@@@@@@@ ",
    " @@@@@@@@@  @@@@  @@@@@@@@@@ ",
    " @@@@@@@@@@  @@  @@@@@@@@@@ ",
    "  @@@@@@@@@@    @@@@@@@@@@ ",
    "   @@@@@@@@@@   @@@@@@@@@ ",
    "    @@@@@@@@@@@@@@@@@@@@ ",
];

const LABEL_WIDTH: usize = 9;

fn main() {
    let username = current_username();
    let hostname = read_trimmed("/proc/sys/kernel/hostname")
        .filter(|h| !h.is_empty() && h != "(none)")
        .unwrap_or_else(|| "void".to_string());
    let kernel = read_trimmed("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".to_string());
    let uptime = read_uptime_secs().map(format_uptime).unwrap_or_else(|| "unknown".to_string());
    let packages = count_packages();
    let cpu = read_cpu_model().unwrap_or_else(|| "unknown".to_string());
    let memory = read_memory_mib()
        .map(|(used, total)| format!("{used}MiB / {total}MiB"))
        .unwrap_or_else(|| "unknown".to_string());

    let header = format!("{}@{}", username, hostname);
    let info: Vec<String> = vec![
        header.red().bold().to_string(),
        "-".repeat(header.chars().count()).dimmed().to_string(),
        labeled("OS:", "VoidOS (Linux, GNU-free)"),
        labeled("Kernel:", &kernel),
        labeled("Uptime:", &uptime),
        labeled("Packages:", &packages.to_string()),
        labeled("Shell:", "vsh"),
        labeled("Editor:", "bit"),
        labeled("CPU:", &cpu),
        labeled("Memory:", &memory),
    ];

    println!();
    for i in 0..LOGO.len().max(info.len()) {
        let art = LOGO.get(i).copied().unwrap_or("").cyan();
        let text = info.get(i).cloned().unwrap_or_default();
        println!(" {art}  {text}");
    }
    println!();

    print!("  ");
    for color in [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ] {
        print!("{}", "   ".on_color(color));
    }
    println!("\n");
}

// Pads the plain label first, then colorizes just that piece -- coloring
// a combined "label + value" string and trying to re-color a substring
// back out afterwards is fragile (nothing stops `value` from matching
// elsewhere, or from containing characters that confuse the escape
// sequences already wrapping the whole string).
fn labeled(label: &str, value: &str) -> String {
    let label = format!("{label:<LABEL_WIDTH$}").cyan().bold();
    format!("{label} {value}")
}

#[derive(Deserialize, Default)]
struct UserData {
    #[serde(default)]
    current_user: String,
}

fn current_username() -> String {
    fs::read_to_string("/etc/userspace.toml")
        .ok()
        .and_then(|text| toml::from_str::<UserData>(&text).ok())
        .map(|u| u.current_user)
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "void".to_string())
}

fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_uptime_secs() -> Option<u64> {
    read_trimmed("/proc/uptime")?
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .map(|f| f as u64)
}

fn format_uptime(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn count_packages() -> usize {
    fs::read_dir("/var/lib/void/installed")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
                .count()
        })
        .unwrap_or(0)
}

fn read_cpu_model() -> Option<String> {
    let text = fs::read_to_string("/proc/cpuinfo").ok()?;
    text.lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
}

fn read_memory_mib() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = None;
    let mut available = None;
    for line in text.lines() {
        if let Some(v) = parse_meminfo_kb(line, "MemTotal:") {
            total = Some(v);
        } else if let Some(v) = parse_meminfo_kb(line, "MemAvailable:") {
            available = Some(v);
        }
    }
    let total = total?;
    let used = total.saturating_sub(available.unwrap_or(0));
    Some((used / 1024, total / 1024))
}

fn parse_meminfo_kb(line: &str, prefix: &str) -> Option<u64> {
    let rest = line.strip_prefix(prefix)?;
    rest.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_uptime_under_an_hour_as_minutes_only() {
        assert_eq!(format_uptime(300), "5m");
    }

    #[test]
    fn formats_uptime_over_an_hour_with_both_units() {
        assert_eq!(format_uptime(3 * 3600 + 90), "3h 1m");
    }

    #[test]
    fn parses_meminfo_kb_lines() {
        let meminfo = "MemTotal:        8048576 kB\nMemAvailable:    4048576 kB\n";
        assert_eq!(parse_meminfo_kb(meminfo.lines().next().unwrap(), "MemTotal:"), Some(8048576));
    }

    #[test]
    fn parses_cpu_model_from_proc_cpuinfo_text() {
        let cpuinfo = "processor\t: 0\nmodel name\t: AMD QEMU Virtual CPU version 2.5+\ncache size\t: 512 KB\n";
        let model = cpuinfo
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string());
        assert_eq!(model.as_deref(), Some("AMD QEMU Virtual CPU version 2.5+"));
    }
}
