use std::fs;

fn main() {
    // Reads from /proc rather than calling uname(2) directly, so this
    // stays a plain std::fs read with no Unix-only compile requirement.
    let kernel_version = fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();
    let machine = std::env::consts::ARCH;

    println!("Linux {kernel_version} {machine}");
}
