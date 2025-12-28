use std::path::Path;

fn main() {
    // Get arguments
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    // Check arguments
    if args.is_empty() {
        eprintln!("No arguments were specified!");
        std::process::exit(1);
    }

    for arg in args {
        // Check if file exists
        if Path::new(&arg).exists() {
            eprintln!("A file with that name already exists.");
            continue;
        }

        // Create file
        match std::fs::File::create(&arg) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error when creating the file: {}", e);
            }
        }
    }


}