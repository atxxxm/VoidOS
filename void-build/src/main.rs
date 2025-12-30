use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

// C //
const INIT_PATH: &str = "../dev/C/init/build/";
// LIBRARIES
const VEC_LIB_PATH: &str = "../dev/C/libraries/void_vec/build";
const VEC_H_PATH: &str = "../dev/C/libraries/void_vec/void_vec.h";
const STRING_LIB_PATH: &str = "../dev/C/libraries/void_string/build";
const STRING_H_PATH: &str = "../dev/C/libraries/void_string/void_string.h";
const FILE_LIB_PATH: &str = "../dev/C/libraries/void_file/build";
const FILE_H_PATH: &str = "../dev/C/libraries/void_file/void_file.h";
// DEMONS
const DEMONS_PATH: &str = "../dev/C/demons/build";
// DEMON CONTROLLER
const DEMON_CONTROLLER_PATH: &str = "../dev/C/Demon Controller/build";
const DEMON_CONTROLLER_LIB_PATH: &str = "../dev/C/Demon Controller/library/lib";
const DEMON_CONTROLLER_INCLUDE_PATH: &str = "../dev/C/Demon Controller/library/include";

// RUST //
// Filesystem
const FILESYSTEM_PATH: &str = "../dev/Rust/filesystem";
const UTILS_PATH: &str = "../dev/Rust/utils";
const VSH_PATH: &str = "../dev/Rust/vsh";

// PATH OS //
const ROOTFS_REL_PATH: &str = "../os/rootfs";

fn main() -> std::io::Result<()> {
    let original_dir = std::env::current_dir()?;
    let rootfs = original_dir.join(ROOTFS_REL_PATH);

    println!("Start building...");
    println!("Rootfs path: {:?}", rootfs);

    // --- Create os dirs ---
    fs::create_dir_all(&rootfs)?;
    fs::create_dir_all(rootfs.join("bin"))?;
    fs::create_dir_all(rootfs.join("sbin"))?;
    fs::create_dir_all(rootfs.join("etc/demons"))?;
    
    let demons_d_path = rootfs.join("etc/demons.d");
    fs::File::create(&demons_d_path)?;

    // write syslog demon in etc/demons.d
    fs::write(&demons_d_path, "syslog")?;

    // --- BUILD and COPY PROGRAMS ---

    // === Build Init ===
    println!("Build Init...");
    let init_build_dir = original_dir.join(INIT_PATH);
    fs::create_dir_all(&init_build_dir)?;
    
    std::env::set_current_dir(&init_build_dir)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    
    fs::copy("init", rootfs.join("init"))?;
    println!("Build Init done!");

    std::env::set_current_dir(&original_dir)?;

    // === Build Demons ===
    println!("Build demons...");
    let demons_build_dir = original_dir.join(DEMONS_PATH);
    fs::create_dir_all(&demons_build_dir)?;
    
    std::env::set_current_dir(&demons_build_dir)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    
    fs::copy("syslog", rootfs.join("etc/demons/syslog"))?;
    println!("Build demons done!");

    std::env::set_current_dir(&original_dir)?;

    // === Build Libraries ===
    println!("Build libraries...");
    
    // Vec
    println!("Build vec_file...");
    let vec_lib_dir = original_dir.join(VEC_LIB_PATH);
    fs::create_dir_all(&vec_lib_dir)?;
    std::env::set_current_dir(&vec_lib_dir)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    
    std::env::set_current_dir(&original_dir)?;

    // String
    println!("Build string_file...");
    let string_lib_dir = original_dir.join(STRING_LIB_PATH);
    fs::create_dir_all(&string_lib_dir)?;
    std::env::set_current_dir(&string_lib_dir)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    
    std::env::set_current_dir(&original_dir)?;

    // File
    println!("Build file_file...");
    let file_lib_dir = original_dir.join(FILE_LIB_PATH);
    fs::create_dir_all(&file_lib_dir)?;
    std::env::set_current_dir(&file_lib_dir)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    println!("Build libraries done!");

    std::env::set_current_dir(&original_dir)?;

    // === Build Demon Controller ===
    println!("Build Demon Controller...");
    
    let dc_include_path = original_dir.join(DEMON_CONTROLLER_INCLUDE_PATH);
    let dc_lib_path = original_dir.join(DEMON_CONTROLLER_LIB_PATH);
    let dc_build_path = original_dir.join(DEMON_CONTROLLER_PATH);

    fs::create_dir_all(&dc_include_path)?;
    fs::create_dir_all(&dc_lib_path)?;

    fs::copy(vec_lib_dir.join("libvoid_vec.a"), dc_lib_path.join("libvoid_vec.a"))?;
    fs::copy(string_lib_dir.join("libvoid_string.a"), dc_lib_path.join("libvoid_string.a"))?;
    fs::copy(file_lib_dir.join("libvoid_file.a"), dc_lib_path.join("libvoid_file.a"))?;

    fs::copy(original_dir.join(VEC_H_PATH), dc_include_path.join("void_vec.h"))?;
    fs::copy(original_dir.join(STRING_H_PATH), dc_include_path.join("void_string.h"))?;
    fs::copy(original_dir.join(FILE_H_PATH), dc_include_path.join("void_file.h"))?;

    let target_h_path = dc_include_path.join("void_file.h");
    let void_file_text = fs::read_to_string(&target_h_path)?;
    let mut new_text = String::new();

    for line in void_file_text.lines() {
        if line.trim() == "#include \"../void_string/void_string.h\"" {
            new_text.push_str("#include \"void_string.h\"\n");
            continue;
        }
        new_text.push_str(&format!("{}\n", line));
    }
    fs::write(target_h_path, new_text)?;

    fs::create_dir_all(&dc_build_path)?;
    std::env::set_current_dir(&dc_build_path)?;
    Command::new("cmake").arg("..").status()?;
    Command::new("cmake").arg("--build").arg(".").status()?;
    
    fs::copy("demon-controller", rootfs.join("sbin/demon-controller"))?;
    println!("Build Demon Controller done!");

    std::env::set_current_dir(&original_dir)?;

    // === Build VSH (Rust) ===
    println!("Build vsh...");
    let vsh_path = original_dir.join(VSH_PATH);
    std::env::set_current_dir(&vsh_path)?;
    Command::new("cargo")
        .arg("b")
        .arg("--release")
        .arg("--target=x86_64-unknown-linux-musl")
        .status()?;

    let vsh_bin_src = vsh_path.join("target/x86_64-unknown-linux-musl/release/vsh");
    fs::copy(&vsh_bin_src, rootfs.join("bin/vsh"))?;
    println!("Build vsh done!");

    std::env::set_current_dir(&original_dir)?;

    // === Build Filesystem (Rust) ===
    println!("Build filesystem...");
    let fs_path = original_dir.join(FILESYSTEM_PATH);
    std::env::set_current_dir(&fs_path)?;
    
    Command::new("cargo")
        .arg("b")
        .arg("--release")
        .arg("--target=x86_64-unknown-linux-musl")
        .status()?;

    println!("Build filesystem done!");
    
    if let Ok(entries) = fs::read_dir("bin") {
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(file_name) = path.file_stem() {
                if let Some(file_name_str) = file_name.to_str() {
                    let src = format!("target/x86_64-unknown-linux-musl/release/{}", file_name_str);
                    let dest = rootfs.join("bin").join(file_name_str);
                    // Check if file exists before copying
                    if Path::new(&src).exists() {
                        fs::copy(src, dest)?;
                    } else {
                        eprintln!("Warning: Binary {} not found in target dir", file_name_str);
                    }
                }
            }
        }
    }

    std::env::set_current_dir(&original_dir)?;

    // === Build Utils (Rust) ===
    println!("Build utils...");
    let utils_path = original_dir.join(UTILS_PATH);
    std::env::set_current_dir(&utils_path)?;
    
    Command::new("cargo")
        .arg("b")
        .arg("--release")
        .arg("--target=x86_64-unknown-linux-musl")
        .status()?;

    if let Ok(entries) = fs::read_dir("bin") {
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(file_name) = path.file_stem() {
                if let Some(file_name_str) = file_name.to_str() {
                     let src = format!("target/x86_64-unknown-linux-musl/release/{}", file_name_str);
                     let dest = rootfs.join("bin").join(file_name_str);
                     if Path::new(&src).exists() {
                        fs::copy(src, dest)?;
                     }
                }
            }
        }
    }
    println!("Build utils done!");

    // === Create Initramfs ===
    std::env::set_current_dir(&rootfs)?;

    println!("Create initramfs.cpio...");
    let mut find_child = Command::new("find")
        .arg(".")
        .stdout(Stdio::piped())
        .spawn()?;

    let cpio_path = rootfs.parent().unwrap().join("initramfs.cpio");
    let output_file = fs::File::create(cpio_path)?;

    let mut cpio_child = Command::new("cpio")
        .args(["-o", "-H", "newc"])
        .stdin(find_child.stdout.take().unwrap())
        .stdout(output_file)
        .spawn()?;

    let find_status = find_child.wait()?;
    let cpio_status = cpio_child.wait()?;

    if !find_status.success() || !cpio_status.success() {
        eprintln!("Error find or cpio");
        std::process::exit(1);
    }

    println!("Create initramfs.cpio done!");

    Ok(())
}