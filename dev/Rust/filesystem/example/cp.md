# Copy a file  
cpfile source.txt dest.txt

# Copy a file into a directory  
cpfile source.txt ./backup/

# Recursively copy a directory  
cpfile -r my_folder/ backup_folder/

# Recursively copy a directory using the long option  
cpfile --recursive my_folder/ backup_folder/