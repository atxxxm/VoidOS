#include "void_string.h"

#include <stdlib.h> 
#include <string.h>  
#include <stdio.h>   
#include <ctype.h>   
#include <stdarg.h>

// Initial capacity
#define STRING_INITIAL_CAP 16


VString VString_new(void) {
    // Allocate memory for the VString
    VString s = {0};
    s.cap = STRING_INITIAL_CAP;
    s.data = malloc(s.cap);

    // Check if allocation was successful
    if (s.data == NULL) {
        s.cap = 0;
        return s;
    }

    // Initialize the string
    s.data[0] = '\0';
    s.len = 0;
    return s;
}

VString VString_from(const char *str) {
    // Handle NULL input
    VString s = {0};

    // Handle empty string
    if (str == NULL || *str == '\0') {
        s.len = 0;
        s.cap = 0;
        s.data = malloc(s.cap);

        // Check if allocation was successful
        if (s.data != NULL) {
            s.data[0] = '\0';
        }

        return s;
    }

    // Get length of the input string
    size_t len = strlen(str);

    // Allocate memory for the VString
    s.len = len;
    s.cap = len + 1;
    s.data = malloc(s.cap * sizeof(char));

    // Copy the string
    if (s.data != NULL) {
        strcpy(s.data, str);
    }
    else {
        // Handle allocation failure
        s.len = 0;
        s.cap = 0;
    }
}

void VString_free(VString *str) {
    if (str != NULL) {
        // Free the allocated memory
        free(str->data);
        str->data = NULL;
        str->len = 0;
        str->cap = 0;
    }
}

int VString_push_str(VString *str, const char *suffix) {
    // Handle NULL input
    if (str == NULL || suffix == NULL || *suffix == '\0') {
        return 0;
    }

    // Get length of the suffix
    size_t suffix_len = strlen(suffix);
    size_t new_len = str->len + suffix_len;
    size_t required_cap = new_len + 1;

    // Check if we need to reallocate
    if (required_cap > str->cap) {
        // Reallocate memory
        size_t new_cap = required_cap * 2;
        char *new_data = realloc(str->data, new_cap * sizeof(char));

        // Check if reallocation was successful
        if (new_data == NULL) {
            return -1;
        }

        // Update the string
        str->data = new_data;
        str->cap = new_cap;
    }

    // Append the suffix
    strcpy(str->data + str->len, suffix);
    str->len = new_len;

    return 1;
}

int VString_push_char(VString *str, char c) {
    if (str == NULL) {
        return 0;
    }

    size_t new_len = str->len + 1;
    size_t required_cap = new_len + 1;

    // Check if we need to reallocate
    if (required_cap > str->cap) {
        size_t new_cap = required_cap * 2;
        char *new_data = realloc(str->data, new_cap * sizeof(char));

        // Check if reallocation was successful
        if (new_data == NULL) {
            return -1;
        }

        // Update the string
        str->data = new_data;
        str->cap = new_cap;
    }

    // Append the character
    str->data[str->len] = c;
    str->data[new_len] = '\0';
    str->len = new_len;

    return 1;
}

int VString_pop(VString *str) {
    if (str == NULL || str->data == NULL || str->len == 0) return 0;

    // Remove the last character
    str->len--;
    str->data[str->len] = '\0';

    return 1;
}

char VString_get(VString *str, int idx) {
    // Handle NULL input
    if (str == NULL || str->data == NULL) return ' ';
    if (idx < 0 || idx >= (int)str->len) return ' ';

    // Get the character
    return str->data[idx];
}

int VString_insert(VString *str, char ch, int idx) {
    if (str == NULL || str ->data == NULL) return 0;
    if (idx < 0 || idx > (int)str->len) return 0;

    // Increase capacity
    size_t new_line = str->len + 1;
    size_t required_cap = new_line + 1;

    // Check if we need to reallocate
    if (required_cap > str->cap) {
        // Reallocate memory
        size_t new_cap = required_cap * 2;
        char *new_data = realloc(str->data, new_cap * sizeof(char));

        // Check if reallocation was successful
        if (new_data == NULL) return 0;

        // Update the string
        str->data = new_data;
        str->cap = new_cap;
    }

    // Insert the character
    for (int i = str->len; i > idx; i--) {
        str->data[i] = str->data[i - 1];
    }

    /// Insert the character
    str->data[idx] = ch;
    str->len = new_line;
    str->data[str->len] = '\0';

    return 1;
}

void VString_clear_fast(VString *str) {
    if (str == NULL || str->data == NULL) return;

    // Clear the string
    str->len = 0;
    str->data[0] = '\0';
}

void VString_clear_full(VString *str) {
    if (str == NULL) return;

    // Clear the string and free memory
    free(str->data);
    str->data = NULL;
    str->len = 0;
    str->cap = 0;
}

size_t VString_len(const VString *str) {
    if (str == NULL) return (size_t)-1;

    return str->len;
}

bool VString_is_empty(const VString *str) {
    if (str == NULL) return true;
    if (str->data == NULL) return true;
    return str->len == 0;
}

const char *VString_cstr(const VString *str) {
    if (str == NULL || str->data == NULL) return "";
    return str->data;
}

bool VString_cmp(const VString *a, const VString *b) {
    // Handle NULL inputs
    if (a == NULL || b == NULL) return false;
    if (a->len != b->len) return false;
    if (a->data == NULL || b->data == NULL) return false;

    // Compare the strings
    return strcmp(a->data, b->data) == 0;
}

bool VString_cmp_cstr(const VString *a, const char *b) {
    if (a == NULL || b == NULL || a->data == NULL) return false;

    // Compare the strings
    return strcmp(a->data, b) == 0;
}

size_t VString_find_char(const VString *str, char ch) {
    if (str == NULL || str->data == NULL) return (size_t)-1;

    // Find the character
    for (size_t i = 0; i < str->len; i++) {
        if (str->data[i] == ch) return i;
    }

    // Not found
    return (size_t)-1;
}

size_t VString_find_str(const VString *str, const char *substr) {
    if (str == NULL || str->data == NULL || substr == NULL) return (size_t)-1;
    if (*substr == '\0') return 0;

    // Find the substring
    char *found = strstr(str->data, substr);
    if (found == NULL) return (size_t)-1;

    // Return the index of the first character
    return (size_t)(found - str->data);
}

bool VString_strip_suffix(VString *str, const char *suffix) {
    if (str == NULL || str->data == NULL || suffix == NULL) return false;

    size_t suffix_len = strlen(suffix);
    size_t str_len = str->len;

    // Check if the suffix matches the end of the string
    if (str_len >= suffix_len && strncmp(str->data + str_len - suffix_len, suffix, suffix_len) == 0) {
        str->len = str_len - suffix_len;
        str->data[str->len] = '\0';
        return true;
    }

    return false;
}

bool VString_strip_prefix(VString *str, const char *prefix) {
    if (str == NULL || str->data == NULL || prefix == NULL) return false;
    
    // Get the length of the prefix
    size_t prefix_len = strlen(prefix);
    
    // Check if the prefix matches the start of the string
    if (str->len >= prefix_len && strncmp(str->data, prefix, prefix_len) == 0) {
        memmove(str->data, str->data + prefix_len, str->len - prefix_len + 1);
        str->len -= prefix_len;
        return true;
    }
    
    return false;
}

int VString_push_fmt(VString *str, const char *fmt, ...) {
    if (str == NULL || fmt == NULL) return 0;
    
    // Use va_list to handle variable arguments
    va_list args;
    va_start(args, fmt);
    
    // Use vsnprintf to calculate the required size
    va_list args_copy;
    va_copy(args_copy, args);
    int needed = vsnprintf(NULL, 0, fmt, args_copy);
    va_end(args_copy);
    
    // Check if the formatting was successful
    if (needed < 0) {
        va_end(args);
        return 0;
    }
    
    // Ensure there's enough space
    size_t new_len = str->len + needed;
    size_t required_cap = new_len + 1;
    
    // Resize if needed
    if (required_cap > str->cap) {
        size_t new_cap = required_cap * 2;
        char *new_data = realloc(str->data, new_cap);
        // Check if realloc was successful
        if (new_data == NULL) {
            va_end(args);
            return 0;
        }

        // Update the string
        str->data = new_data;
        str->cap = new_cap;
    }
    
    // Write the formatted string
    int written = vsnprintf(str->data + str->len, needed + 1, fmt, args);
    va_end(args);
    
    // Check if the write was successful
    if (written >= 0) {
        str->len += written;
        return written;
    }
    
    return 0;
}

int VString_replace_first(VString *str, const char *old, const char *new_str) {
    if (str == NULL || str->data == NULL || old == NULL || new_str == NULL) return 0;
    
    // Get lengths
    size_t old_len = strlen(old);
    size_t new_len = strlen(new_str);
    
    // Check if old is empty
    if (old_len == 0) return 0;
    
    // Find the substring
    char *found = strstr(str->data, old);
    if (found == NULL) return 0;
    
    // Calculate positions
    size_t pos = found - str->data;
    size_t new_total_len = str->len - old_len + new_len;
    
    // Resize if needed
    if (new_total_len + 1 > str->cap) {
        size_t new_cap = (new_total_len + 1) * 2;
        char *new_data = realloc(str->data, new_cap);

        // Check if realloc was successful
        if (new_data == NULL) return 0;

        // Update the string
        str->data = new_data;
        str->cap = new_cap;
        found = str->data + pos;
    }
    
    // Move the rest of the string
    if (new_len != old_len) {
        memmove(found + new_len, found + old_len, str->len - pos - old_len + 1);
    }
    
    // Copy the new string
    memcpy(found, new_str, new_len);
    str->len = new_total_len;
    
    return 1;
}

int VString_push_int(VString *str, int value) {
    return VString_push_fmt(str, "%d", value);
}

int VString_push_float(VString *str, float value) {
    return VString_push_fmt(str, "%.2f", value);
}

bool VString_starts_with(const VString *str, const char *prefix) {
    if (str == NULL || str->data == NULL || prefix == NULL) return false;
    
    // Check if the prefix is longer than the string
    size_t prefix_len = strlen(prefix);
    if (prefix_len > str->len) return false;
    
    // Compare the prefix with the start of the string
    return strncmp(str->data, prefix, prefix_len) == 0;
}

bool VString_ends_with(const VString *str, const char *suffix) {
    if (str == NULL || str->data == NULL || suffix == NULL) return false;
    
    // Check if the suffix is longer than the string
    size_t suffix_len = strlen(suffix);
    if (suffix_len > str->len) return false;
    
    // Compare the suffix with the end of the string
    return strncmp(str->data + str->len - suffix_len, suffix, suffix_len) == 0;
}

void VString_trim_start(VString *str) {
    if (str == NULL || str->data == NULL) return;
    
    // Find the first non-space character
    size_t start = 0;
    while (start < str->len && isspace((unsigned char)str->data[start])) {
        start++;
    }
    
    // Move the data if needed
    if (start > 0) {
        memmove(str->data, str->data + start, str->len - start + 1);
        str->len -= start;
    }
}

void VString_trim_end(VString *str) {
    if (str == NULL || str->data == NULL) return;
    
    // Find the last non-space character
    size_t end = str->len;
    while (end > 0 && isspace((unsigned char)str->data[end - 1])) {
        end--;
    }
    
    // Truncate the string if needed 
    if (end < str->len) {
        str->data[end] = '\0';
        str->len = end;
    }
}

void VString_trim(VString *str) {
    VString_trim_start(str);
    VString_trim_end(str);
}

VString VString_clone(const VString *str) {
    VString clone = {0};
    
    if (str == NULL || str->data == NULL) return clone;
    
    // Allocate memory for the clone
    clone.len = str->len;
    clone.cap = str->len + 1;
    clone.data = malloc(clone.cap);
    
    // Copy the data
    if (clone.data != NULL) {
        strcpy(clone.data, str->data);
    } else {
        // Handle allocation failure
        clone.len = 0;
        clone.cap = 0;
    }
    
    return clone;
}

bool VString_valid(const VString *str) {
    return str != NULL && str->data != NULL && str->cap > 0;
}

VString VString_from_n(const char *str, size_t len) {
    VString vstr = {0};
    
    // Handle NULL or empty string
    if (str == NULL || len == 0) {
        vstr.cap = 1;
        vstr.data = malloc(vstr.cap);
        if (vstr.data != NULL) {
            vstr.data[0] = '\0';
        }
        return vstr;
    }
    
    // Truncate if needed
    size_t actual_len = strlen(str);
    if (len > actual_len) {
        len = actual_len;
    }
    
    // Allocate memory
    vstr.len = len;
    vstr.cap = len + 1;
    vstr.data = malloc(vstr.cap);
    
    // Copy the data
    if (vstr.data != NULL) {
        strncpy(vstr.data, str, len);
        vstr.data[len] = '\0';
    } else {
        // Handle allocation failure
        vstr.len = 0;
        vstr.cap = 0;
    }
    
    return vstr;
}

void VString_truncate(VString *str, size_t new_len) {
    if (!VString_valid(str) || new_len >= str->len) return;
    
    // Truncate
    str->len = new_len;
    str->data[str->len] = '\0';
}

VString VString_substr(const VString *str, size_t start, size_t len) {
    VString result = {0};
    
    // Handle invalid input
    if (!VString_valid(str) || start >= str->len) {
        return result;
    }
    
    // Adjust length if needed
    if (start + len > str->len) {
        len = str->len - start;
    }
    
    // Allocate memory
    result.len = len;
    result.cap = len + 1;
    result.data = malloc(result.cap);
    
    // Copy the data
    if (result.data != NULL) {
        strncpy(result.data, str->data + start, len);
        result.data[len] = '\0';
    } else {
        // Handle allocation failure
        result.len = 0;
        result.cap = 0;
    }
    
    return result;
}

void VString_to_lowercase(VString *str) {
    if (!VString_valid(str)) return;

    // Convert to lowercase
    for (size_t i = 0; i < str->len; i++) {
        str->data[i] = tolower((unsigned char)str->data[i]);
    }
}

void VString_to_uppercase(VString *str) {
    if (!VString_valid(str)) return;
    
    // Convert to uppercase
    for (size_t i = 0; i < str->len; i++) {
        str->data[i] = toupper((unsigned char)str->data[i]);
    }
}

bool VString_is_digit(const VString *str) {
    if (!VString_valid(str) || str->len == 0) return false;
    
    // Check each character is a digit
    for (size_t i = 0; i < str->len; i++) {
        if (!isdigit((unsigned char)str->data[i])) {
            return false;
        }
    }
    
    return true;
}

bool VString_is_hex(const VString *str) {
    if (!VString_valid(str) || str->len == 0) return false;
    
    // Check each character is a hex digit
    for (size_t i = 0; i < str->len; i++) {
        char c = str->data[i];
        if (!(isdigit((unsigned char)c) || 
              (c >= 'a' && c <= 'f') || 
              (c >= 'A' && c <= 'F'))) {
            return false;
        }
    }
    
    return true;
}

int VString_replace_all(VString *str, const char *old, const char *new_str) {
    if (!VString_valid(str) || old == NULL || new_str == NULL) return 0;
    
    // Get lengths
    size_t old_len = strlen(old);
    size_t new_len = strlen(new_str);
    
    // Handle empty old string
    if (old_len == 0) return 0;
    
    // Find and replace
    int replacements = 0;
    size_t pos = 0;
    
    // Use strstr for efficient substring search
    while (pos <= str->len - old_len) {
        char *found = strstr(str->data + pos, old);
        if (found == NULL) break;
        
        // Calculate positions
        size_t found_pos = found - str->data;
        
        size_t new_total_len = str->len - old_len + new_len;
        
        // Reallocate if needed
        if (new_total_len + 1 > str->cap) {
            size_t new_cap = (new_total_len + 1) * 2;
            char *new_data = realloc(str->data, new_cap);

            // Handle allocation failure
            if (new_data == NULL) break;

            // Update pointers
            str->data = new_data;
            str->cap = new_cap;
            found = str->data + found_pos;
        }
        
        // Move data
        if (new_len != old_len) {
            memmove(found + new_len, found + old_len, str->len - found_pos - old_len + 1);
        }
        
        // Copy new string
        memcpy(found, new_str, new_len);
        str->len = new_total_len;
        
        // Update positions
        replacements++;
        pos = found_pos + new_len; 
    }
    
    return replacements;
}
