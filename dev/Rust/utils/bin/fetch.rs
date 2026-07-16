use std::{
    io::{Read, Write},
    net::TcpStream,
};
use clap::Parser;

// A deliberately minimal HTTP client: http:// only (no TLS -- that's a
// much bigger undertaking than fits here), no redirect following, and no
// chunked-transfer-encoding awareness (relies on "Connection: close" so
// the server just closes the socket when it's done, which covers plain
// HTTP/1.0-ish responses and most HTTP/1.1 servers when asked to close).
#[derive(Parser)]
#[command(version, about = "Minimal HTTP/1.1 client (http:// only, no TLS)", long_about = None)]
struct Args {
    url: String,

    /// Save the response body to this file instead of printing it
    #[arg(short = 'o', long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let Some(parsed) = parse_url(&args.url) else {
        eprintln!("fetch: invalid URL '{}' (expected http://host[:port]/path)", args.url);
        std::process::exit(1);
    };

    match do_fetch(&parsed) {
        Ok(body) => {
            if let Some(path) = &args.output {
                if let Err(e) = std::fs::write(path, &body) {
                    eprintln!("fetch: cannot write '{path}': {e}");
                    std::process::exit(1);
                }
                eprintln!("Saved {} bytes to {path}", body.len());
            } else {
                std::io::stdout().write_all(&body).ok();
            }
        }
        Err(e) => {
            eprintln!("fetch: {e}");
            std::process::exit(1);
        }
    }
}

struct Url {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Option<Url> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], rest[idx..].to_string()),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()?),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return None;
    }
    Some(Url { host, port, path })
}

fn do_fetch(url: &Url) -> std::io::Result<Vec<u8>> {
    // Hostname resolution goes through std's own resolver, which on Unix
    // is backed by libc's getaddrinfo -- so this only resolves real
    // hostnames (not just literal IPs) if the NSS DNS module and
    // /etc/resolv.conf are present in the rootfs.
    let mut stream = TcpStream::connect((url.host.as_str(), url.port))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: void-fetch/0.1\r\nConnection: close\r\n\r\n",
        url.path, url.host
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    let split_at = find_header_end(&response).unwrap_or(response.len());
    let (headers, body) = response.split_at(split_at);
    let status_line = headers.split(|&b| b == b'\n').next().unwrap_or(b"");
    eprintln!("{}", String::from_utf8_lossy(status_line).trim());

    Ok(body.to_vec())
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_url_with_explicit_port_and_path() {
        let u = parse_url("http://example.com:8080/foo/bar").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 8080);
        assert_eq!(u.path, "/foo/bar");
    }

    #[test]
    fn defaults_port_80_and_root_path() {
        let u = parse_url("http://example.com").unwrap();
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 80);
        assert_eq!(u.path, "/");
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(parse_url("https://example.com").is_none());
        assert!(parse_url("ftp://example.com").is_none());
    }

    #[test]
    fn finds_the_header_body_boundary() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi";
        let idx = find_header_end(data).unwrap();
        assert_eq!(&data[idx..], b"hi");
    }
}
