use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

const TARGET: &str = "x86_64-unknown-linux-musl";

const INIT_PATH: &str = "../dev/Rust/init";
const DEMON_CONTROLLER_PATH: &str = "../dev/Rust/demon-controller";
const DEMONS_PATH: &str = "../dev/Rust/demons";
const VSH_PATH: &str = "../dev/Rust/vsh";
const FILESYSTEM_PATH: &str = "../dev/Rust/filesystem";
const UTILS_PATH: &str = "../dev/Rust/utils";
const ROOTFS_REL_PATH: &str = "../os/rootfs";

fn build_crate(path: &Path) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    println!("Building {name}...");

    let status = Command::new("cargo")
        .args(["zigbuild", "--release", &format!("--target={TARGET}")])
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
