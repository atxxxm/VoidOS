use std::fs;

fn main() {
    println!("{:>6} {:<8} CMD", "PID", "STATE");

    let Ok(entries) = fs::read_dir("/proc") else {
        eprintln!("ps: cannot read /proc");
        std::process::exit(1);
    };

    let mut pids: Vec<u32> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    pids.sort_unstable();

    for pid in pids {
        let comm = fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let state = fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| parse_state(&s))
            .unwrap_or('?');

        println!("{pid:>6} {state:<8} {comm}");
    }
}

// /proc/[pid]/stat's third field (right after the ")"-terminated comm) is
// the state letter -- parsed manually since comm itself may contain
// spaces or parentheses.
fn parse_state(stat: &str) -> Option<char> {
    let after_comm = stat.rsplit_once(')')?.1;
    after_comm.trim_start().chars().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_state_letter_after_a_parenthesized_comm() {
        assert_eq!(parse_state("42 (my proc) S 1 42 42"), Some('S'));
    }

    #[test]
    fn handles_a_comm_that_itself_contains_parentheses() {
        assert_eq!(parse_state("7 ((weird)) R 1 7 7"), Some('R'));
    }
}
