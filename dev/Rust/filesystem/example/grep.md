# Find lines containing "error" in a file  
grep "error" log.txt

# Ignore case  
grep -i "warning" log.txt

# Recursively search throughout a directory  
grep -r "TODO" src/

# Count the number of matches  
grep -c "fn" src/lib.rs

# Show lines that do **not** contain "debug"  
grep -v "debug" log.txt

# Highlight matches  
grep --color "panic" main.rs

# Show line numbers  
grep -n "unsafe" src/

# Search from standard input  
echo -e "line1\nerror here\nline3" | grep "error"

# Combine flags: recursive, with line numbers, case-insensitive  
grep -r -n -i "FIXME" ./project/