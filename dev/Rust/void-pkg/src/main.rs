mod db;
mod manifest;
mod tar_reader;

use std::{fs, path::Path};

use anyhow::Context;
use clap::{Parser, Subcommand};

use db::InstalledPackage;
use manifest::Manifest;

// void: a package manager for a system with neither a network stack nor
// persistent storage (yet). Scope for now: install/remove/list/info
// against a local .tar package file -- nothing is fetched from anywhere,
// and since the rootfs lives in tmpfs, installed packages don't survive a
// reboot any more than anything else does. Network-based fetching is a
// natural follow-up once there's an actual TCP/IP stack.
#[derive(Parser)]
#[command(name = "void", version, about = "VoidOS package manager", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install a package from a local .tar archive
    Install {
        archive: String,
        /// Reinstall even if already installed
        #[arg(long)]
        force: bool,
    },
    /// Remove an installed package
    Remove { name: String },
    /// List installed packages
    List,
    /// Show details about an installed package
    Info { name: String },
}

fn main() {
    let args = Args::parse();
    let root = Path::new("/");
    let db_dir = Path::new(db::DEFAULT_DB_DIR);

    let result = match args.command {
        Command::Install { archive, force } => install(&archive, force, root, db_dir),
        Command::Remove { name } => remove(&name, db_dir),
        Command::List => list(db_dir),
        Command::Info { name } => info(&name, db_dir),
    };

    if let Err(e) = result {
        eprintln!("void: {e}");
        std::process::exit(1);
    }
}

fn install(archive_path: &str, force: bool, root: &Path, db_dir: &Path) -> anyhow::Result<()> {
    let data = fs::read(archive_path).with_context(|| format!("cannot read '{archive_path}'"))?;

    let mut manifest: Option<Manifest> = None;
    let mut file_entries: Vec<(String, Vec<u8>)> = Vec::new();

    tar_reader::for_each_entry(&data, |entry, content| {
        if entry.name == "void.toml" {
            manifest = Some(toml::from_str(&String::from_utf8_lossy(content))?);
        } else if let Some(rel) = entry.name.strip_prefix("files/")
            && !rel.is_empty()
            && entry.typeflag != b'5'
            && !rel.ends_with('/')
        {
            file_entries.push((rel.to_string(), content.to_vec()));
        }
        Ok(())
    })?;

    let manifest = manifest.context("archive has no void.toml manifest")?;

    if db::is_installed(db_dir, &manifest.name) && !force {
        anyhow::bail!(
            "{} is already installed (use --force to reinstall)",
            manifest.name
        );
    }

    let missing: Vec<&String> = manifest
        .dependencies
        .iter()
        .filter(|dep| !db::is_installed(db_dir, dep))
        .collect();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        eprintln!(
            "warning: {} depends on {} which {} not installed",
            manifest.name,
            names.join(", "),
            if missing.len() == 1 { "is" } else { "are" }
        );
    }

    let mut installed_files = Vec::new();
    for (rel, content) in &file_entries {
        let target = root.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
        installed_files.push(target.to_string_lossy().into_owned());
    }

    println!(
        "Installed {} {} ({} file(s))",
        manifest.name,
        manifest.version,
        installed_files.len()
    );

    db::save(db_dir, &InstalledPackage { manifest, files: installed_files })
}

fn remove(name: &str, db_dir: &Path) -> anyhow::Result<()> {
    let pkg = db::load(db_dir, name).with_context(|| format!("{name} is not installed"))?;

    // Warn (don't block) if something still depends on it -- there's no
    // hard dependency enforcement here, just visibility.
    let dependents: Vec<String> = db::list_installed(db_dir)?
        .into_iter()
        .filter(|p| p.manifest.dependencies.contains(&name.to_string()))
        .map(|p| p.manifest.name)
        .collect();
    if !dependents.is_empty() {
        eprintln!("warning: {name} is still depended on by: {}", dependents.join(", "));
    }

    let mut removed = 0;
    for file in &pkg.files {
        if fs::remove_file(file).is_ok() {
            removed += 1;
        }
    }
    db::remove_record(db_dir, name)?;

    println!("Removed {name} ({removed} file(s))");
    Ok(())
}

fn list(db_dir: &Path) -> anyhow::Result<()> {
    let packages = db::list_installed(db_dir)?;
    if packages.is_empty() {
        println!("No packages installed");
        return Ok(());
    }
    for pkg in packages {
        println!("{} {}", pkg.manifest.name, pkg.manifest.version);
    }
    Ok(())
}

fn info(name: &str, db_dir: &Path) -> anyhow::Result<()> {
    let pkg = db::load(db_dir, name).with_context(|| format!("{name} is not installed"))?;
    println!("Name: {}", pkg.manifest.name);
    println!("Version: {}", pkg.manifest.version);
    if !pkg.manifest.description.is_empty() {
        println!("Description: {}", pkg.manifest.description);
    }
    if !pkg.manifest.dependencies.is_empty() {
        println!("Dependencies: {}", pkg.manifest.dependencies.join(", "));
    }
    println!("Files ({}):", pkg.files.len());
    for file in &pkg.files {
        println!("  {file}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const BLOCK: usize = 512;

    fn write_header(out: &mut Vec<u8>, name: &str, size: usize, typeflag: u8) {
        let mut header = [0u8; BLOCK];
        header[0..name.len()].copy_from_slice(name.as_bytes());
        let mode = format!("{:0>7o}\0", 0o644);
        header[100..100 + mode.len()].copy_from_slice(mode.as_bytes());
        let size_str = format!("{size:0>11o}\0");
        header[124..124 + size_str.len()].copy_from_slice(size_str.as_bytes());
        header[156] = typeflag;
        header[257..257 + 6].copy_from_slice(b"ustar\0");
        header[263..263 + 2].copy_from_slice(b"00");
        for b in &mut header[148..156] {
            *b = b' ';
        }
        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        let checksum_str = format!("{checksum:0>6o}\0 ");
        header[148..148 + 8].copy_from_slice(checksum_str.as_bytes());
        out.extend_from_slice(&header);
    }

    fn write_entry(out: &mut Vec<u8>, name: &str, content: &[u8]) {
        write_header(out, name, content.len(), b'0');
        out.write_all(content).unwrap();
        let pad = (BLOCK - (content.len() % BLOCK)) % BLOCK;
        out.write_all(&vec![0u8; pad]).unwrap();
    }

    // Builds a minimal void package archive: a void.toml manifest plus one
    // payload file under files/.
    fn build_test_package(name: &str, version: &str, payload_rel: &str, payload: &[u8]) -> Vec<u8> {
        let manifest_toml = format!("name = \"{name}\"\nversion = \"{version}\"\n");
        let mut out = Vec::new();
        write_entry(&mut out, "void.toml", manifest_toml.as_bytes());
        write_entry(&mut out, &format!("files/{payload_rel}"), payload);
        out.extend_from_slice(&[0u8; BLOCK * 2]);
        out
    }

    struct TestEnv {
        dir: std::path::PathBuf,
    }

    impl TestEnv {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("void_pkg_test_{tag}"));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        fn root(&self) -> std::path::PathBuf {
            self.dir.join("root")
        }

        fn db_dir(&self) -> std::path::PathBuf {
            self.dir.join("db")
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn installs_a_package_and_records_it() {
        let env = TestEnv::new("install");
        let archive = build_test_package("hello", "1.0.0", "bin/hello", b"#!/bin/vsh\necho hi\n");
        let archive_path = env.dir.join("hello.tar");
        fs::write(&archive_path, &archive).unwrap();

        install(archive_path.to_str().unwrap(), false, &env.root(), &env.db_dir()).unwrap();

        let installed_file = env.root().join("bin/hello");
        assert!(installed_file.exists());
        assert_eq!(fs::read(&installed_file).unwrap(), b"#!/bin/vsh\necho hi\n");

        let pkg = db::load(&env.db_dir(), "hello").unwrap();
        assert_eq!(pkg.manifest.version, "1.0.0");
        assert_eq!(pkg.files.len(), 1);
    }

    #[test]
    fn refuses_to_reinstall_without_force() {
        let env = TestEnv::new("reinstall");
        let archive = build_test_package("hello", "1.0.0", "bin/hello", b"v1");
        let archive_path = env.dir.join("hello.tar");
        fs::write(&archive_path, &archive).unwrap();

        install(archive_path.to_str().unwrap(), false, &env.root(), &env.db_dir()).unwrap();
        let result = install(archive_path.to_str().unwrap(), false, &env.root(), &env.db_dir());
        assert!(result.is_err());

        // --force (`true` here) should succeed.
        install(archive_path.to_str().unwrap(), true, &env.root(), &env.db_dir()).unwrap();
    }

    #[test]
    fn remove_deletes_installed_files_and_the_db_record() {
        let env = TestEnv::new("remove");
        let archive = build_test_package("hello", "1.0.0", "bin/hello", b"v1");
        let archive_path = env.dir.join("hello.tar");
        fs::write(&archive_path, &archive).unwrap();
        install(archive_path.to_str().unwrap(), false, &env.root(), &env.db_dir()).unwrap();

        remove("hello", &env.db_dir()).unwrap();

        assert!(!env.root().join("bin/hello").exists());
        assert!(!db::is_installed(&env.db_dir(), "hello"));
    }

    #[test]
    fn list_reports_installed_packages_sorted_by_name() {
        let env = TestEnv::new("list");
        for (name, payload_rel) in [("zeta", "bin/zeta"), ("alpha", "bin/alpha")] {
            let archive = build_test_package(name, "1.0.0", payload_rel, b"x");
            let archive_path = env.dir.join(format!("{name}.tar"));
            fs::write(&archive_path, &archive).unwrap();
            install(archive_path.to_str().unwrap(), false, &env.root(), &env.db_dir()).unwrap();
        }

        let packages = db::list_installed(&env.db_dir()).unwrap();
        let names: Vec<&str> = packages.iter().map(|p| p.manifest.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
    }
}
