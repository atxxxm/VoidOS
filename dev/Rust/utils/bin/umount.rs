use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Unmount a filesystem", long_about = None)]
struct Args {
    target: String,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = do_umount(&args.target) {
        eprintln!("umount: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn do_umount(target: &str) -> std::io::Result<()> {
    use std::ffi::CString;
    let c_target = CString::new(target)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { libc::umount(c_target.as_ptr()) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn do_umount(_target: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "umount is only supported on Unix",
    ))
}
