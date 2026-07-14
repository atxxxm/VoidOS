use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

// Base triple; this is also the name of the directory cargo/zigbuild puts
// artifacts under (target/<TARGET>/release), regardless of the glibc
// version suffix passed to --target below.
const TARGET: &str = "x86_64-unknown-linux-gnu";

// Versioned target passed to `cargo zigbuild`: pins the minimum glibc ABI
// binaries require (2.31 ~ Ubuntu 20.04 / Debian 11) for broad compatibility.
const ZIG_TARGET: &str = "x86_64-unknown-linux-gnu.2.31";

// Directory holding real glibc runtime shared objects (libc.so.6,
// ld-linux-x86-64.so.2, ...) to bundle into the rootfs. Zig only provides
// link-time stubs, not runtime libraries, so this must be populated by hand
// (e.g. copied out of a real glibc-based Linux system) before building.
const GLIBC_SYSROOT_PATH: &str = "../os/glibc-sysroot";

const INIT_PATH: &str = "../dev/Rust/init";
const DEMON_CONTROLLER_PATH: &str = "../dev/Rust/demon-controller";
const DEMONS_PATH: &str = "../dev/Rust/demons";
const VSH_PATH: &str = "../dev/Rust/vsh";
const BIT_PATH: &str = "../dev/Rust/bit";
const FILESYSTEM_PATH: &str = "../dev/Rust/filesystem";
const UTILS_PATH: &str = "../dev/Rust/utils";
const ROOTFS_REL_PATH: &str = "../os/rootfs";

fn build_crate(path: &Path) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    println!("Building {name}...");

    // Embed an RPATH pointing at /lib64 so the dynamic linker finds our
    // bundled glibc there regardless of which distro built it (Debian's
    // ld.so defaults to /lib/x86_64-linux-gnu, not /lib64, and we have no
    // ld.so.cache to widen its search).
    let status = Command::new("cargo")
        .args(["zigbuild", "--release", &format!("--target={ZIG_TARGET}")])
        .env("RUSTFLAGS", "-C link-arg=-Wl,-rpath,/lib64")
        .current_dir(path)
        .status()
        .unwrap_or_else(|e| panic!("failed to run cargo in {path:?}: {e}"));

    if !status.success() {
        eprintln!("Build failed: {name}");
        std::process::exit(1);
    }

    println!("Built {name}.");
}

fn release_bin(crate_path: &Path, bin: &str) -> PathBuf {
    crate_path
        .join("target")
        .join(TARGET)
        .join("release")
        .join(bin)
}

fn copy_bin(src: &Path, dest: &Path) -> std::io::Result<()> {
    if src.exists() {
        fs::copy(src, dest)?;
    } else {
        eprintln!("Warning: binary not found: {src:?}");
    }
    Ok(())
}

fn bins_in_dir(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect()
}

// Copy every regular file from the glibc sysroot dir (ld-linux, libc.so.6,
// libm.so.6, ...) into rootfs/lib64. The sysroot isn't produced by this
// build: zig only supplies link-time stubs, so someone has to populate
// GLIBC_SYSROOT_PATH by hand from a real glibc-based Linux system first.
fn copy_glibc_sysroot(sysroot: &Path, dest: &Path) -> std::io::Result<()> {
    if !sysroot.exists() {
        eprintln!(
            "Warning: glibc sysroot not found at {sysroot:?} — binaries will be \
             dynamically linked but missing libc.so.6 / ld-linux-x86-64.so.2 at \
             boot. Populate that directory before running VoidOS."
        );
        return Ok(());
    }

    for entry in fs::read_dir(sysroot)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name();
            fs::copy(&path, dest.join(&name))?;
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let base = std::env::current_dir()?;
    let rootfs = base.join(ROOTFS_REL_PATH);

    println!("Start building...");
    println!("Rootfs: {rootfs:?}");

    fs::create_dir_all(rootfs.join("bin"))?;
    fs::create_dir_all(rootfs.join("sbin"))?;
    fs::create_dir_all(rootfs.join("etc/demons"))?;
    // Mount points for init (proc, sys, tmp, dev)
    fs::create_dir_all(rootfs.join("proc"))?;
    fs::create_dir_all(rootfs.join("sys"))?;
    fs::create_dir_all(rootfs.join("tmp"))?;
    fs::create_dir_all(rootfs.join("dev"))?;
    fs::create_dir_all(rootfs.join("home"))?;
    // Dynamic linker + glibc runtime libs (interpreter path baked into
    // our binaries is /lib64/ld-linux-x86-64.so.2)
    fs::create_dir_all(rootfs.join("lib64"))?;

    fs::write(rootfs.join("etc/demons.d"), "syslog\ncrond")?;
    // Default empty crontab (lines: min hour mday mon wday command)
    fs::write(
        rootfs.join("etc/crontab"),
        "# min hour mday mon wday command\n",
    )?;

    // init
    let init_path = base.join(INIT_PATH);
    build_crate(&init_path);
    copy_bin(&release_bin(&init_path, "init"), &rootfs.join("init"))?;

    // demon-controller
    let dc_path = base.join(DEMON_CONTROLLER_PATH);
    build_crate(&dc_path);
    copy_bin(
        &release_bin(&dc_path, "demon-controller"),
        &rootfs.join("sbin/demon-controller"),
    )?;

    // demons: crond + syslog
    let demons_path = base.join(DEMONS_PATH);
    build_crate(&demons_path);
    copy_bin(
        &release_bin(&demons_path, "crond"),
        &rootfs.join("etc/demons/crond"),
    )?;
    copy_bin(
        &release_bin(&demons_path, "syslog"),
        &rootfs.join("etc/demons/syslog"),
    )?;

    // vsh
    let vsh_path = base.join(VSH_PATH);
    build_crate(&vsh_path);
    copy_bin(&release_bin(&vsh_path, "vsh"), &rootfs.join("bin/vsh"))?;

    // bit (ratatui text editor)
    let bit_path = base.join(BIT_PATH);
    build_crate(&bit_path);
    copy_bin(&release_bin(&bit_path, "bit"), &rootfs.join("bin/bit"))?;

    // filesystem tools (cat, cp, find, grep, ls, mkdir, mv, rm, rmdir, touch)
    let fs_path = base.join(FILESYSTEM_PATH);
    build_crate(&fs_path);
    for bin in bins_in_dir(&fs_path.join("bin")) {
        copy_bin(
            &release_bin(&fs_path, &bin),
            &rootfs.join("bin").join(&bin),
        )?;
    }

    // utils (usereg, void)
    let utils_path = base.join(UTILS_PATH);
    build_crate(&utils_path);
    for bin in bins_in_dir(&utils_path.join("bin")) {
        copy_bin(
            &release_bin(&utils_path, &bin),
            &rootfs.join("bin").join(&bin),
        )?;
    }

    // glibc runtime (ld-linux-x86-64.so.2, libc.so.6, ...)
    copy_glibc_sysroot(&base.join(GLIBC_SYSROOT_PATH), &rootfs.join("lib64"))?;

    // Create initramfs
    println!("Creating initramfs.cpio...");
    let cpio_path = rootfs.parent().unwrap().join("initramfs.cpio");
    cpio::write(&rootfs, &cpio_path)?;

    println!("Build complete!");
    Ok(())
}

// ── Newc cpio writer ────────────────────────────────────────────────────────

mod cpio {
    use super::*;

    pub fn write(rootfs: &Path, out_path: &Path) -> io::Result<()> {
        let mut out = io::BufWriter::new(fs::File::create(out_path)?);
        let mut ino: u32 = 1;

        for entry in WalkDir::new(rootfs).sort_by_file_name() {
            let entry = entry.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let path = entry.path();
            let rel = path
                .strip_prefix(rootfs)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let rel = if rel.is_empty() { ".".to_string() } else { rel };

            write_entry(&mut out, path, &rel, ino)?;
            ino += 1;
        }

        write_trailer(&mut out)
    }

    fn file_mode(path: &Path, rel: &str) -> u32 {
        if path.is_dir() {
            return 0o040755;
        }
        if rel == "init"
            || rel.starts_with("bin/")
            || rel.starts_with("sbin/")
            || rel.starts_with("etc/demons/")
            || rel.starts_with("lib64/")
        {
            return 0o100755;
        }
        0o100644
    }

    fn write_entry(w: &mut impl Write, path: &Path, rel: &str, ino: u32) -> io::Result<()> {
        let mode = file_mode(path, rel);
        let data: Option<Vec<u8>> = if path.is_file() {
            Some(fs::read(path)?)
        } else if path.is_dir() {
            None
        } else {
            return Ok(());
        };

        let filesize = data.as_ref().map_or(0, |d| d.len());
        let name_bytes = rel.as_bytes();
        let namesize = name_bytes.len() + 1;

        write!(
            w,
            "070701{ino:08X}{mode:08X}{uid:08X}{gid:08X}{nlink:08X}{mtime:08X}\
             {filesize:08X}{devmaj:08X}{devmin:08X}{rdevmaj:08X}{rdevmin:08X}\
             {namesize:08X}{check:08X}",
            ino = ino,
            mode = mode,
            uid = 0u32,
            gid = 0u32,
            nlink = 1u32,
            mtime = 0u32,
            filesize = filesize as u32,
            devmaj = 0u32,
            devmin = 1u32,
            rdevmaj = 0u32,
            rdevmin = 0u32,
            namesize = namesize as u32,
            check = 0u32,
        )?;

        w.write_all(name_bytes)?;
        w.write_all(&[0])?;
        pad4(w, 110 + namesize)?;

        if let Some(data) = data {
            w.write_all(&data)?;
            pad4(w, filesize)?;
        }

        Ok(())
    }

    fn write_trailer(w: &mut impl Write) -> io::Result<()> {
        let name = "TRAILER!!!";
        let namesize = name.len() + 1;
        write!(
            w,
            "070701{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}{:08X}",
            0u32, 0u32, 0u32, 0u32, 1u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
            namesize as u32, 0u32,
        )?;
        w.write_all(name.as_bytes())?;
        w.write_all(&[0])?;
        pad4(w, 110 + namesize)
    }

    fn pad4(w: &mut impl Write, len: usize) -> io::Result<()> {
        let pad = (4 - (len % 4)) % 4;
        w.write_all(&[0u8; 3][..pad])
    }
}
