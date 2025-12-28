# Find lines containing "error" in a file  
ffgrep "error" log.txt

# Ignore case  
ffgrep -i "warning" log.txt

# Recursively search throughout a directory  
ffgrep -r "TODO" src/

# Count the number of matches  
ffgrep -c "fn" src/lib.rs

# Show lines that do **not** contain "debug"  
ffgrep -v "debug" log.txt

# Highlight matches  
ffgrep --color "panic" main.rs

# Show line numbers  
ffgrep -n "unsafe" src/

# Search from standard input  
echo -e "line1\nerror here\nline3" | ffgrep "error"

# Combine flags: recursive, with line numbers, case-insensitive  
ffgrep -r -n -i "FIXME" ./project/