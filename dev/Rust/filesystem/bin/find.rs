use std::{fs, path::{Path, PathBuf}};
use chrono::{DateTime, Local, NaiveDate};
use anyhow;
use rayon::prelude::*;
use clap::Parser;
use walkdir::WalkDir;
use globset::{Glob, GlobSet, GlobSetBuilder};

#[derive(Parser)]
#[command(version, about = "Advanced file finder")]
struct Args {
    /// Path to start searching from
    #[arg(default_value = ".")]
    path: String,

    /// Filename pattern (e.g. *.rs, data/*.txt)
    name: Option<String>,

    /// Match names exactly (--exact)
    #[arg(long)]
    exact: bool,

    /// Ignore case when matching (--ignore-case)
    #[arg(long)]
    ignore_case: bool,

    /// Maximum search depth (--max-depth 3)
    #[arg(long, default_value_t = usize::MAX)]
    max_depth: usize,

    /// Search for files only
    #[arg(short = 'f', long)]
    search_files: bool,

    /// Search for directories only
    #[arg(short = 'd', long)]
    search_dirs: bool,

    /// Minimum file size in bytes
    #[arg(long)]
    min_size: Option<u64>,

    /// Maximum file size in bytes
    #[arg(long)]
    max_size: Option<u64>,

    /// Filter: modified after (YYYY-MM-DD)
    #[arg(long)]
    modified_after: Option<String>,

    /// Filter: modified before (YYYY-MM-DD)
    #[arg(long)]
    modified_before: Option<String>,

    /// Use parallel directory traversal
    #[arg(long)]
    parallel: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let path = Path::new(&args.path);

    if !path.exists() {
        eprintln!("Path not found: {}", args.path);
        std::process::exit(1);
    }

    // Default to searching both files and directories
    let flag_files = args.search_files || (!args.search_dirs && !args.search_files);
    let flag_dirs = args.search_dirs || (!args.search_dirs && !args.search_files);

    let globset = build_globset(&args)?;

    // Directory traversal parrallel or not
    if args.parallel {
        parallel_walk(path, &args, &globset, flag_files, flag_dirs)?;
    } else {
        walk(path, &args, &globset, flag_files, flag_dirs)?;
    }

    Ok(())
}


// Build globset
fn build_globset(args: &Args) -> anyhow::Result<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();

    if let Some(pattern) = &args.name {
        if args.ignore_case {
            builder.add(Glob::new(&pattern.to_lowercase())?);
            builder.add(Glob::new(&pattern.to_uppercase())?);
        } else {
            builder.add(Glob::new(pattern)?);
        }
    }

    Ok(builder.build()?)
}

// Directory traversal
fn walk(root: &Path, args: &Args, globset: &GlobSet, flag_files: bool, flag_dirs: bool) -> anyhow::Result<()> {
    for entry in WalkDir::new(root)
        .max_depth(args.max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok() ) {

            let path = entry.path();

            if matches_entry(path, args, globset, flag_files, flag_dirs)? {
                println!("{}", path.display());
            }
        }
    Ok(())
}

// Parallel directory traversal
fn parallel_walk(root: &Path, args: &Args, globset: &GlobSet, flag_files: bool, flag_dirs: bool) -> anyhow::Result<()> {
    let entries: Vec<PathBuf> = fs::read_dir(root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();

    entries.par_iter().for_each(|path| {
        let _ = walk(path, args, globset, flag_files, flag_dirs);
    });

    Ok(())
}

// Entry matching
fn matches_entry(path: &Path, args: &Args, globset: &GlobSet, flag_files: bool, flag_dirs: bool) -> anyhow::Result<bool> {
    // Type
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(false)
    };

    let is_file = meta.is_file();
    let is_dir = meta.is_dir();

    if (is_file && !flag_files) || (is_dir && !flag_dirs) {
        return  Ok(false);
    }

    // Name
    if let Some(file_name_os) = path.file_name() {
        let name = file_name_os.to_string_lossy();

        // --exact or glob
        if args.exact {
            let mut cmp_name = name.to_string();
            let mut cmp_pattern = args.name.clone().unwrap_or_default();
            
            if args.ignore_case {
                cmp_name = cmp_name.to_lowercase();
                cmp_pattern = cmp_pattern.to_lowercase();
            }

            if cmp_name != cmp_pattern {
                return Ok(false);
            }

        } else if !args.name.is_none() && !globset.is_match(path) {
            return Ok(false);
        }
    }

    // Size and time
    if !matches_filters(path, args)? {
        return Ok(false);
    }

    Ok(true)
}

// File filters
fn matches_filters(path: &Path, args: &Args) -> anyhow::Result<bool> {
    let meta = fs::metadata(path)?;
    let size = meta.len();

    if let Some(min) = args.min_size {
        if size < min {
            return Ok(false);
        }
    }

    if let Some(max) = args.max_size {
        if size > max {
            return Ok(false);
        }
    }

    // Time
    if let Ok(modified) = meta.modified() {
        let modified: DateTime<Local> = modified.into();

        // --modified-after
        if let Some(after_str) = &args.modified_after {
            if let Ok(after) = NaiveDate::parse_from_str(after_str, "%Y-%m-%d") {
                if modified.date_naive() < after {
                    return Ok(false);
                }
            }
        }

        // --modified-before
        if let Some(before_str) = &args.modified_before {
            if let Ok(before) = NaiveDate::parse_from_str(before_str, "%Y-%m-%d") {
                if modified.date_naive() > before {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}
