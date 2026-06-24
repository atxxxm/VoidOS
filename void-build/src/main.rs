use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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
        .args(["build", "--release", &format!("--target={TARGET}")])
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
    fs::write(rootfs.join("etc/demons.d"), "syslog\ncrond")?;

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

    let mut find = Command::new("find")
        .arg(".")
        .current_dir(&rootfs)
        .stdout(Stdio::piped())
        .spawn()?;

    let cpio_file = fs::File::create(&cpio_path)?;
    let mut cpio = Command::new("cpio")
        .args(["-o", "-H", "newc"])
        .stdin(find.stdout.take().unwrap())
        .stdout(cpio_file)
        .spawn()?;

    if !find.wait()?.success() || !cpio.wait()?.success() {
        eprintln!("Error creating initramfs");
        std::process::exit(1);
    }

    println!("Build complete!");
    Ok(())
}
