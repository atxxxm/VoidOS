use std::fs;
use clap::Parser;
use colored::*;
use time::OffsetDateTime;
use time::macros::format_description;

#[derive(Parser)]
#[command(version, about = "List of catalog contents", long_about = None)]
struct Args {
    /// Show hidden files
    #[arg(short = 'a', long)]
    all: bool,

    /// Detailed output
    #[arg(short = 'l', long)]
    long: bool,

    /// The path to the directory (by default - the current one)
    #[arg(default_value = ".")]
    path: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // If no path is specified, use the current directory
    let paths = if args.path.is_empty() {
        vec![".".to_string()]
    } else {
        args.path
    };

    let more_than_one = paths.len() > 1;

    for path in paths {
        read_dir(path, args.all, args.long, more_than_one);
    }
}


// Read directory
fn read_dir(current_path: String, all: bool, long: bool, more_than_one: bool) {
    let mut data: Vec<(String, std::fs::Metadata)> = Vec::new();

    match fs::read_dir(&current_path) {
        Ok(entries) => {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Get the file name
                let name = entry.file_name().to_string_lossy().to_string();

                if name.starts_with('.') && !all { continue; }

                // Get metadata
                if let Ok(metadata) = entry.metadata() {
                    data.push((name, metadata,));
                } else {
                    continue;
                };
            }
        }

        Err(e) => {
            eprintln!("Cannot open directory: '{}': {}", current_path, e);
            return;
        }

    }

    data.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    if more_than_one {  println!("{}:", current_path); }

    if long {
        for (name, meta) in data {
            let size = meta.len();

            // Get the last modified time
            let mtime_str = if let Ok(mtime) = meta.modified() {
                // Comvert the time to a string
                if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                    let secs = dur.as_secs() as i64;
                    format_time(secs)
                } else {
                    "?".to_string()
                }
            } else {
                "?".to_string()
            };

            // Colorize the name
            let colored_name = if meta.is_dir() {
                name.cyan()
            } else if meta.is_file() {
                name.white()
            } else {
                name.normal()
            };

            println!("{:>12} | {} | {}", size, mtime_str, colored_name);
        }
    } else {
        // Normal mode
        let colored_names: Vec<String> = data.into_iter().map(|(name, meta)| {
            if meta.is_dir() {
                name.cyan().to_string()
            } else if meta.is_file() {
                name.white().to_string()
            } else {
                name.normal().to_string()
            }
        }).collect();

        for name in colored_names.chunks(5) {
            println!("{}", name.join("   "));
        }
    }

}

// Format time
fn format_time(secs: i64) -> String {
    // Convert to a time
    if let Ok(dt) = OffsetDateTime::from_unix_timestamp(secs) {
        // Format the time
        let format = format_description!("[month repr:short] [day] [hour]:[minute]");
        // Convert to a string
        dt.format(&format).unwrap_or_else(|_| "?".to_string())
    } else {
        "?".to_string()
    }
}