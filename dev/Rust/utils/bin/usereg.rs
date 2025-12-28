use std::{collections::HashMap, path::Path};
use std::io::Read;
use std::io::Write;
use std::fs;
use clap::{Parser, Subcommand};
use toml;
use anyhow::{self, Context};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2
};

const ETC_USERSPACE_PATH: &str = "/etc/userspace.toml";

#[derive(Parser)]
#[command(version, about = "User registration", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new user
    New {

        /// Username
        #[arg(short = 'n', long = "name")]
        username: String,

        /// Password
        #[arg(short = 'p', long = "password")]
        password: String,
    },

    /// Delete a selected user
    Delete {
        /// Username
        #[arg(short = 'n', long = "name")]
        username: String,

        /// Password
        #[arg(short = 'p', long = "password")]
        password: String,
    },

    /// Login user
    Login {
        /// Username
        #[arg(short = 'n', long = "name")]
        username: String,

        /// Password
        #[arg(short = 'p', long = "password")]
        password: String,
    },

    /// Change user password
    Edit {
        #[arg(long = "name", short = 'n')]
        username: String,

        #[arg(long = "old")]
        old_password: String,

        #[arg(long = "new")]
        new_password: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::New { username, password } => {
            add_user(username, password)?;
        }
        Commands::Delete { username, password } => {
            delete_user(username, password)?;
        }
        Commands::Login { username, password } => {
            login_user(username, password)?;
        }
        Commands::Edit { username, old_password, new_password } => {
            edit_user(username, old_password, new_password)?;
        }
    }

    Ok(())
}

// DATA //

// Struct for user data
#[derive(Serialize, Deserialize)]
struct UserData {
    users: HashMap<String, String>,
    current_user: String,
}

// Load data from file about users
fn load_data<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let mut file = std::fs::File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    toml::from_str(&data).context("Failed read userdata!")
}

// Save data to file about users
fn save_data<T: Serialize>(path: &str, data: &T) -> anyhow::Result<()> {
    let toml_string = toml::to_string(data)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(toml_string.as_bytes())?;
    Ok(())
}

// Hash password
fn hash_password(password: &str) -> anyhow::Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?.to_string();

    Ok(password_hash)
}

// Verify password
fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = PasswordHash::new(hash).unwrap();

    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}

// Functions //

// Add user
fn add_user(username: String, password: String) -> anyhow::Result<()> {
    // Check if userspace exists
    if !Path::new(ETC_USERSPACE_PATH).exists() {
        let hash_password = hash_password(&password)?;
        
        // Create user data
        let mut user_data = UserData {
            users: HashMap::new(),
            current_user: String::new(),
        };

        // Add user to user data
        user_data.users.insert(username.clone(), hash_password);

        // Save user data
        save_data(ETC_USERSPACE_PATH, &user_data)?;
        
        println!("User {} successfully created!", username);
        
        return Ok(());
    }

    // Load user data
    let mut user_data: UserData = load_data(ETC_USERSPACE_PATH)?;

    // Check if user already exists
    if user_data.users.contains_key(&username) {
        println!(
            "User {} already exists!",
            username
        );

        return Ok(());
    }

    // Hash password
    let password_hash = hash_password(&password)?;

    // Add user to user data
    user_data.users.insert(username.clone(), password_hash);

    // Save user data
    save_data(ETC_USERSPACE_PATH, &user_data)?;

    create_userspace(&username)?;

    println!("User {} successfully created!", username);

    Ok(())
}

// Delete user
fn delete_user(username: String, password: String) -> anyhow::Result<()> {
    // Check if userspace exists
    if !Path::new(ETC_USERSPACE_PATH).exists() {
        println!("User {} does not exist!", username);
        return Ok(());
    }

    // Load user data
    let mut user_data: UserData = load_data(ETC_USERSPACE_PATH)?;

    // Check if user exists
    if !user_data.users.contains_key(&username) {
        println!(
            "User {} does not exist!",
            username
        );
        return Ok(());
    }

    // Get hash password
    let hash = user_data.users.get(&username).unwrap();

    // Verify password
    if !verify_password(&password, hash) {
        println!(
            "Wrong password for user {}!",
            username
        );
        return Ok(());
    }
    
    // Remove user from user data
    user_data.users.remove(&username).unwrap();

    // Check if user is current user
    if user_data.current_user == username {
        user_data.current_user = String::new();
    }

    // Save user data
    save_data(ETC_USERSPACE_PATH, &user_data)?;
    // Remove userspace
    remove_userspace(&username)?;

    println!("User {} has been deleted!", username);

    Ok(())
}

// Login user
fn login_user(username: String, password: String) -> anyhow::Result<()> {
    // Check if userspace exists
    if !Path::new(ETC_USERSPACE_PATH).exists() {
        println!(
            "User {} does not exist!",
            username
        );
        return Ok(());
    }

    // Load user data
    let mut data: UserData = load_data(ETC_USERSPACE_PATH)?;

    // Check if user exists
    if !data.users.contains_key(&username) {
        println!(
            "User {} does not exist!",
            username
        );
        return Ok(());
    }

    // Get hash password
    let hash = data.users.get(&username).unwrap();

    // Verify password
    if !verify_password(&password, hash) {
        println!(
            "Wrong password for user {}!",
            username
        );
        return Ok(());
    }

    // Set current user
    data.current_user = username.clone();
    // Save user data
    save_data(ETC_USERSPACE_PATH, &data)?;
    println!("Welcome, {}!", username);

    Ok(())
}

// Edit user
fn edit_user(username: String, old_password: String, new_password: String) -> anyhow::Result<()> {
    // Check if userspace exists
    if !Path::new(ETC_USERSPACE_PATH).exists() {
        println!(
            "User {} does not exist!",
            username
        );
        return Ok(());
    }

    // Load user data
    let mut data: UserData = load_data(ETC_USERSPACE_PATH)?;

    // Check if user exists
    if !data.users.contains_key(&username) {
        println!(
            "User {} does not exist!",
            username
        );
        return Ok(());
    }

    // Get hash password
    let hash = data.users.get(&username).unwrap();

    // Verify password
    if !verify_password(&old_password, hash) {
        println!(
            "Wrong password for user {}!",
            username
        );
        return Ok(());
    }

    // Check if new password is the same as the old password
    if old_password == new_password {
        println!(
            "New password cannot be the same as the old password!"
        );
        return Ok(());
    }
    
    // New hash password
    let new_password_hash = hash_password(&new_password)?;

    // Edit user
    data.users.insert(username.clone(), new_password_hash);

    // Save user data
    save_data(ETC_USERSPACE_PATH, &data)?;

    println!("User {} password has been edited!", username);

    Ok(())
}


// Create userspace // 

// Create userspace directory
fn create_userspace(username: &str) -> anyhow::Result<()> {
    let root = &format!("/home/");
    fs::create_dir(&format!("{}{}", root, username))?;

    fs::create_dir_all(&format!("/home/{}/Documents", username))?;
    fs::create_dir_all(&format!("/home/{}/Images", username))?;
    fs::create_dir_all(&format!("/home/{}/Video", username))?;
    fs::create_dir_all(&format!("/home/{}/Downloads", username))?;
    fs::create_dir_all(&format!("/home/{}/Music", username))?;

    Ok(())
}

// Remove userspace 
fn remove_userspace(username: &str) -> anyhow::Result<()> {
    let root = &format!("/home/");
    fs::remove_dir_all(&format!("{}{}", root, username))?;

    Ok(())
}