use std::{fs, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::manifest::Manifest;

// Records what got installed, so `remove` can clean it back up. Lives
// under a db directory (real default: /var/lib/void/installed) that's
// passed in explicitly rather than hardcoded, so tests can point it at a
// throwaway temp directory instead of touching a real path.
#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledPackage {
    #[serde(flatten)]
    pub manifest: Manifest,
    pub files: Vec<String>,
}

pub const DEFAULT_DB_DIR: &str = "/var/lib/void/installed";

fn db_path(db_dir: &Path, name: &str) -> PathBuf {
    db_dir.join(format!("{name}.toml"))
}

pub fn is_installed(db_dir: &Path, name: &str) -> bool {
    db_path(db_dir, name).exists()
}

pub fn load(db_dir: &Path, name: &str) -> anyhow::Result<InstalledPackage> {
    let text = fs::read_to_string(db_path(db_dir, name))?;
    Ok(toml::from_str(&text)?)
}

pub fn save(db_dir: &Path, pkg: &InstalledPackage) -> anyhow::Result<()> {
    fs::create_dir_all(db_dir)?;
    let text = toml::to_string_pretty(pkg)?;
    fs::write(db_path(db_dir, &pkg.manifest.name), text)?;
    Ok(())
}

pub fn remove_record(db_dir: &Path, name: &str) -> anyhow::Result<()> {
    let path = db_path(db_dir, name);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn list_installed(db_dir: &Path) -> anyhow::Result<Vec<InstalledPackage>> {
    let mut packages = Vec::new();
    if !db_dir.exists() {
        return Ok(packages);
    }
    for entry in fs::read_dir(db_dir)? {
        let entry = entry?;
        if let Ok(text) = fs::read_to_string(entry.path())
            && let Ok(pkg) = toml::from_str(&text)
        {
            packages.push(pkg);
        }
    }
    packages.sort_by(|a: &InstalledPackage, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(packages)
}
