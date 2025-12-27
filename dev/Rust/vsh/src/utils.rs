
use std::{fs::File, io::{Read, BufReader, BufRead}, path::Path};
use anyhow;

const USERSPACE_PATH: &str = "/etc/userspace.toml";

// Get current username
pub fn get_current_username() -> anyhow::Result<String> {
    if !Path::new(USERSPACE_PATH).exists() {
        return Ok("void".to_string());
    }

    let mut file = File::open(USERSPACE_PATH)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;

    for txt in text.lines() {
        if txt.contains("current") {
            let current_name = txt.split('=').nth(1).unwrap().trim();
            let name = current_name.replace('"', "");
            return Ok(name);
        }
    }

    Ok("void".to_string())
}

// Split command and arguments
pub fn split_cmd_and_args(cmd: &str) -> (String, Vec<String>) {
    let split_t: Vec<&str> = cmd.split_whitespace().collect();
    let cmd = split_t[0].to_string();
    let mut args: Vec<String> = Vec::new();

    for i in 1..split_t.len() {
        args.push(split_t[i].to_string());
    }

    (cmd, args)
}

// Get current path
pub fn get_current_path() -> String {
    let mut current_path = String::new();

    if let Ok(cur_path) = std::env::current_dir() {
        current_path = cur_path.display().to_string();
    }

    current_path
}

// Check if a file is a script
pub fn is_script(path: &str) -> anyhow::Result<bool> {
    if !Path::new(path).exists() {
        return Ok(false);
    }

    let lines = read_first_lines(path, 5)?;

    for line in lines {
        if line.starts_with("#!") && line.contains("vsh") {
            return Ok(true);
        }
    }

    Ok(false)
}

// Reading a specified number of initial lines in a file
fn read_first_lines(filename: &str, n: usize) -> std::io::Result<Vec<String>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    reader.lines().take(n).collect::<Result<Vec<_>, _>>()
}