use std::path::{Path, PathBuf};

fn main() {
    let names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        eprintln!("which: missing operand");
        std::process::exit(1);
    }

    let path_var = std::env::var("PATH").unwrap_or_default();
    let dirs: Vec<&str> = path_var.split(':').filter(|d| !d.is_empty()).collect();

    let mut not_found = false;
    for name in &names {
        match find_in_path(name, &dirs) {
            Some(p) => println!("{}", p.display()),
            None => {
                eprintln!("which: no {name} in ({path_var})");
                not_found = true;
            }
        }
    }

    if not_found {
        std::process::exit(1);
    }
}

fn find_in_path(name: &str, dirs: &[&str]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_no_directory_has_the_file() {
        assert_eq!(find_in_path("definitely-not-a-real-binary", &["/bin", "/usr/bin"]), None);
    }
}
