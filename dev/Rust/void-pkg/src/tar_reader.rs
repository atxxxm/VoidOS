// A minimal, read-only ustar walker -- just enough to pull a manifest and
// payload files back out of an archive built by (for example) this
// project's own `tar -c`. No compression, no long-name prefix support;
// matches the same scope as filesystem/bin/tar.rs's writer.

pub struct Entry {
    pub name: String,
    pub size: u64,
    pub typeflag: u8,
}

const BLOCK: usize = 512;

pub fn for_each_entry(
    data: &[u8],
    mut on_entry: impl FnMut(&Entry, &[u8]) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut offset = 0;

    while offset + BLOCK <= data.len() {
        let header = &data[offset..offset + BLOCK];
        let Some(entry) = read_header(header) else { break };
        offset += BLOCK;

        let content_len = entry.size as usize;
        let content = &data[offset..(offset + content_len).min(data.len())];
        on_entry(&entry, content)?;

        offset += content_len.div_ceil(BLOCK) * BLOCK;
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Builds a tiny valid ustar archive in memory (one file entry) using
    // the same header layout filesystem/bin/tar.rs writes, so this reader
    // can be exercised without shelling out to the real `tar` binary.
    fn build_archive(name: &str, content: &[u8]) -> Vec<u8> {
        let mut header = [0u8; BLOCK];
        header[0..name.len()].copy_from_slice(name.as_bytes());
        let mode = format!("{:0>7o}\0", 0o644);
        header[100..100 + mode.len()].copy_from_slice(mode.as_bytes());
        let size = format!("{:0>11o}\0", content.len());
        header[124..124 + size.len()].copy_from_slice(size.as_bytes());
        header[156] = b'0';
        header[257..257 + 6].copy_from_slice(b"ustar\0");
        header[263..263 + 2].copy_from_slice(b"00");
        for b in &mut header[148..156] {
            *b = b' ';
        }
        let checksum: u32 = header.iter().map(|&b| b as u32).sum();
        let checksum_str = format!("{checksum:0>6o}\0 ");
        header[148..148 + 8].copy_from_slice(checksum_str.as_bytes());

        let mut out = Vec::new();
        out.write_all(&header).unwrap();
        out.write_all(content).unwrap();
        let pad = (BLOCK - (content.len() % BLOCK)) % BLOCK;
        out.write_all(&vec![0u8; pad]).unwrap();
        out.write_all(&[0u8; BLOCK * 2]).unwrap();
        out
    }

    #[test]
    fn reads_back_a_single_entry() {
        let archive = build_archive("hello.txt", b"hi there");
        let mut seen = Vec::new();
        for_each_entry(&archive, |entry, content| {
            seen.push((entry.name.clone(), content.to_vec()));
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, vec![("hello.txt".to_string(), b"hi there".to_vec())]);
    }
}
