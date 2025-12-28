# Find all files and folders in the current directory  
find .

# Find files with the `.rs` extension  
find --name "*.rs"

# Find files whose names contain "main" (case-insensitive)  
find --name "*main*" --ignore-case

# Find only files (excluding directories)  
find -f

# Find only directories  
find -d

# Find files with the exact name "Cargo.toml"  
find --name "Cargo.toml" --exact

# Limit search depth to 2 levels  
find --max-depth 2

# Find files at least 1 KB in size  
find --min-size 1024

# Find files up to 1 MB in size  
find --max-size 1048576

# Find files modified after 2025-01-01  
find --modified-after 2025-01-01

# Find files modified before 2024-12-31  
find --modified-before 2024-12-31

# Combine filters: only .log files, larger than 100 bytes, modified in 2025  
find --name "*.log" -f --min-size 100 --modified-after 2025-01-01 --modified-before 2025-12-31

# Use parallel directory traversal  
find --parallel