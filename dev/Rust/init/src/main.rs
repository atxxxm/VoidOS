use nix::{
    mount::{mount, MsFlags},
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::{execv, fork, ForkResult, Pid},
};
use std::{ffi::CString, fs, thread, time::Duration};

const DEMON_CONTROLLER: &str = "/sbin/demon-controller";
const VSH: &str = "/bin/vsh";

fn do_mount(source: &str, target: &str, fstype: &str) {
    if let Err(e) = mount(
        Some(source),
        target,
        Some(fstype),
        MsFlags::empty(),
        None::<&str>,
    ) {
        eprintln!("[init] mount {target}: {e}");
    }
}

fn spawn_process(path: &str) -> Option<Pid> {
    let cpath = CString::new(path).expect("path contains null byte");
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let _ = execv(cpath.as_c_str(), &[cpath.as_c_str()]);
            eprintln!("[init] execv {path} failed");
            unsafe { libc::_exit(1) };
        }
        Ok(ForkResult::Parent { child }) => {
            println!("[init] started {path} (pid={child})");
            Some(child)
        }
        Err(e) => {
            eprintln!("[init] fork: {e}");
            None
        }
    }
}

fn is_running(pid: Pid) -> bool {
    matches!(
        waitpid(pid, Some(WaitPidFlag::WNOHANG)),
        Ok(WaitStatus::StillAlive)
    )
}

// Reap any orphaned child processes to avoid zombies
fn reap_orphans() {
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => continue,
            _ => break,
        }
    }
}

fn main() {
    println!("VoidOS init starting...");

    do_mount("proc", "/proc", "proc");
    do_mount("sysfs", "/sys", "sysfs");
    do_mount("tmpfs", "/tmp", "tmpfs");
    do_mount("devtmpfs", "/dev", "devtmpfs");

    unsafe { std::env::set_var("PATH", "/bin:/sbin") };
    fs::create_dir_all("/home").ok();

    println!("Initialization complete. Starting shell...");

    let mut dc_pid = spawn_process(DEMON_CONTROLLER);

    loop {
        reap_orphans();

        if let Some(pid) = dc_pid {
            if !is_running(pid) {
                eprintln!("[init] demon-controller exited, restarting...");
                dc_pid = spawn_process(DEMON_CONTROLLER);
            }
        }

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let path = CString::new(VSH).unwrap();
                let _ = execv(path.as_c_str(), &[path.as_c_str()]);
                eprintln!("[init] failed to exec vsh");
                unsafe { libc::_exit(1) };
            }
            Ok(ForkResult::Parent { child }) => {
                let _ = waitpid(child, None);
                println!("[init] shell exited, restarting...");
            }
            Err(e) => {
                eprintln!("[init] fork: {e}");
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}
