use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Mount a filesystem", long_about = None)]
struct Args {
    /// Filesystem type (e.g. tmpfs, proc, sysfs)
    #[arg(short = 't', long)]
    fs_type: Option<String>,

    source: Option<String>,
    target: Option<String>,
}

fn main() {
    let args = Args::parse();

    let (Some(source), Some(target)) = (&args.source, &args.target) else {
        // No source/target given: list current mounts, like real `mount`.
        print_current_mounts();
        return;
    };

    let fs_type = args.fs_type.as_deref().unwrap_or("auto");
    if let Err(e) = do_mount(source, target, fs_type) {
        eprintln!("mount: {e}");
        std::process::exit(1);
    }
}

fn print_current_mounts() {
    match std::fs::read_to_string("/proc/mounts") {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("mount: cannot read /proc/mounts: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(unix)]
fn do_mount(source: &str, target: &str, fs_type: &str) -> std::io::Result<()> {
    use std::ffi::CString;

    let invalid = |_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path");
    let c_source = CString::new(source).map_err(invalid)?;
    let c_target = CString::new(target).map_err(invalid)?;
    let c_fstype = CString::new(fs_type).map_err(invalid)?;

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            0,
            std::ptr::null(),
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn do_mount(_source: &str, _target: &str, _fs_type: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "mount is only supported on Unix",
    ))
}
