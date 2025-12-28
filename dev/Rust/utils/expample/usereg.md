# Add a new user  
usereg new -n alice -p secret123  

# Delete a user (requires password)  
usereg delete -n alice -p secret123  

# Log in as a user  
usereg login -n alice -p secret123  

# Change a user's password  
usereg edit -n alice --old secret123 --new newpass456  

# Create user with long options  
usereg new --name bob --password mypass  

# Delete user with long options  
usereg delete --name bob --password mypass  

# Login with long options  
usereg login --name bob --password mypass  

# Edit password using mixed short/long options  
usereg edit -n bob --old mypass --new betterpass789