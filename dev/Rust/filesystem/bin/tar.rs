use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use clap::Parser;

// A minimal, uncompressed ustar reader/writer: no gzip, no symlinks/
// special files, and the classic ustar 100-byte name field limit (long
// paths get truncated rather than using the prefix-splitting trick real
// tar uses for up to 255 bytes). Covers the common case of archiving a
// directory of regular files.
#[derive(Parser)]
#[command(version, about = "A minimal tar (uncompressed, ustar format only)", long_about = None)]
struct Args {
    /// Create an archive
    #[arg(short = 'c', long)]
    create: bool,

    /// Extract an archive
    #[arg(short = 'x', long)]
    extract: bool,

    /// List archive contents
    #[arg(short = 't', long)]
    list: bool,

    /// Archive file
    #[arg(short = 'f', long)]
    file: String,

    /// Files/directories to add (with -c)
    paths: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let result = if args.create {
        create_archive(&args.file, &args.paths)
    } else if args.extract {
        extract_archive(&args.file)
    } else if args.list {
        list_archive(&args.file)
    } else {
        eprintln!("tar: one of -c, -x, or -t is required");
        std::process::exit(1);
    };

    if let Err(e) = result {
        eprintln!("tar: {e}");
        std::process::exit(1);
    }
}

const BLOCK: usize = 512;

fn create_archive(archive_path: &str, paths: &[String]) -> io::Result<()> {
    if paths.is_empty() {
        eprintln!("tar: no files given to archive");
        std::process::exit(1);
    }
    let mut out = io::BufWriter::new(fs::File::create(archive_path)?);
    for path in paths {
        add_path(&mut out, Path::new(path))?;
    }
    // Two 512-byte zero blocks terminate a tar archive.
    out.write_all(&[0u8; BLOCK * 2])?;
    out.flush()
}

fn add_path(out: &mut impl Write, path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let name = path.to_string_lossy();

    if metadata.is_dir() {
        write_header(out, &format!("{name}/"), 0, 0o755, b'5')?;
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            add_path(out, &entry.path())?;
        }
    } else if metadata.is_file() {
        let data = fs::read(path)?;
        write_header(out, &name, data.len() as u64, 0o644, b'0')?;
        out.write_all(&data)?;
        pad_to_block(out, data.len())?;
    }
    // Symlinks and other special files are silently skipped.
    Ok(())
}

fn write_header(out: &mut impl Write, name: &str, size: u64, mode: u32, typeflag: u8) -> io::Result<()> {
    let mut header = [0u8; BLOCK];

    write_field(&mut header, 0, 100, name.as_bytes());
    write_octal(&mut header, 100, 8, mode as u64);
    write_octal(&mut header, 108, 8, 0); // uid
    write_octal(&mut header, 116, 8, 0); // gid
    write_octal(&mut header, 124, 12, size);
    write_octal(&mut header, 136, 12, 0); // mtime
    header[156] = typeflag;
    write_field(&mut header, 257, 6, b"ustar\0");
    write_field(&mut header, 263, 2, b"00");

    // Checksum is computed with the checksum field itself treated as
    // spaces, then written in last.
    for b in &mut header[148..156] {
        *b = b' ';
    }
    let checksum: u32 = header.iter().map(|&b| b as u32).sum();
    write_octal(&mut header, 148, 8, checksum as u64);
    header[155] = 0;

    out.write_all(&header)
}

fn write_field(header: &mut [u8; BLOCK], offset: usize, len: usize, data: &[u8]) {
    let n = data.len().min(len);
    header[offset..offset + n].copy_from_slice(&data[..n]);
}

fn write_octal(header: &mut [u8; BLOCK], offset: usize, len: usize, value: u64) {
    // len - 1 digits followed by a NUL, right-aligned, zero-padded.
    let s = format!("{:0>width$o}\0", value, width = len - 1);
    write_field(header, offset, len, s.as_bytes());
}

fn pad_to_block(out: &mut impl Write, size: usize) -> io::Result<()> {
    let pad = (BLOCK - (size % BLOCK)) % BLOCK;
    out.write_all(&vec![0u8; pad])
}

struct Entry {
    name: String,
    size: u64,
    typeflag: u8,
}

fn read_header(data: &[u8]) -> Option<Entry> {
    if data.iter().all(|&b| b == 0) {
        return None;
    }
    let name = read_field(data, 0, 100);
    let size = read_octal(data, 124, 12);
    let typeflag = data[156];
    Some(Entry { name, size, typeflag })
}

fn read_field(data: &[u8], offset: usize, len: usize) -> String {
    let raw = &data[offset..offset + len];
    let end = raw.iter().position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

fn read_octal(data: &[u8], offset: usize, len: usize) -> u64 {
    let raw = &data[offset..offset + len];
    let text: String = raw.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
    u64::from_str_radix(text.trim(), 8).unwrap_or(0)
}

fn for_each_entry(
    archive_path: &str,
    mut on_entry: impl FnMut(&Entry, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let data = fs::read(archive_path)?;
    let mut offset = 0;

    while offset + BLOCK <= data.len() {
        let header = &data[offset..offset + BLOCK];
        let Some(entry) = read_header(header) else { break };
        offset += BLOCK;

        let content_len = entry.size as usize;
        let content = &data[offset..(offset + content_len).min(data.len())];
        on_entry(&entry, content)?;

        let padded = content_len.div_ceil(BLOCK) * BLOCK;
        offset += padded;
    }

    Ok(())
}

fn list_archive(archive_path: &str) -> io::Result<()> {
    for_each_entry(archive_path, |entry, _content| {
        println!("{}", entry.name);
        Ok(())
    })
}

fn extract_archive(archive_path: &str) -> io::Result<()> {
    for_each_entry(archive_path, |entry, content| {
        if entry.typeflag == b'5' || entry.name.ends_with('/') {
            fs::create_dir_all(&entry.name)?;
        } else {
            if let Some(parent) = Path::new(&entry.name).parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&entry.name, content)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_single_file_through_create_and_list() {
        let dir = std::env::temp_dir()
            .join("vsh_tar_test")
            .to_string_lossy()
            .replace('\\', "/");
        let _ = fs::create_dir_all(&dir);
        let file_path = format!("{dir}/hello.txt");
        fs::write(&file_path, b"hello tar").unwrap();

        let archive = format!("{dir}/out.tar");
        create_archive(&archive, std::slice::from_ref(&file_path)).unwrap();

        let mut names = Vec::new();
        for_each_entry(&archive, |entry, content| {
            names.push(entry.name.clone());
            if entry.name == file_path {
                assert_eq!(content, b"hello tar");
            }
            Ok(())
        })
        .unwrap();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(names, vec![file_path]);
    }

    #[test]
    fn octal_field_round_trips() {
        let mut header = [0u8; BLOCK];
        write_octal(&mut header, 124, 12, 12345);
        assert_eq!(read_octal(&header, 124, 12), 12345);
    }
}
