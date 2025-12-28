# Find all files and folders in the current directory  
ffind .

# Find files with the `.rs` extension  
ffind --name "*.rs"

# Find files whose names contain "main" (case-insensitive)  
ffind --name "*main*" --ignore-case

# Find only files (excluding directories)  
ffind -f

# Find only directories  
ffind -d

# Find files with the exact name "Cargo.toml"  
ffind --name "Cargo.toml" --exact

# Limit search depth to 2 levels  
ffind --max-depth 2

# Find files at least 1 KB in size  
ffind --min-size 1024

# Find files up to 1 MB in size  
ffind --max-size 1048576

# Find files modified after 2025-01-01  
ffind --modified-after 2025-01-01

# Find files modified before 2024-12-31  
ffind --modified-before 2024-12-31

# Combine filters: only .log files, larger than 100 bytes, modified in 2025  
ffind --name "*.log" -f --min-size 100 --modified-after 2025-01-01 --modified-before 2025-12-31

# Use parallel directory traversal  
ffind --parallel