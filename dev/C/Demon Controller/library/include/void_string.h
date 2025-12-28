#ifndef VOID_STRING_H
#define VOID_STRING_H

#include <stddef.h>
#include <stdbool.h> 
#include <stdarg.h>

typedef struct {
    char *data; // Pointer to buffer (always terminated by ‘\0’)
    size_t len; // Current string length (without ‘\0’)
    size_t cap; // Allocated memory (including space for ‘\0’)
} VString;

// Creates an empty string
VString VString_new(void);

// Creates a string from a C string (copies)
VString VString_from(const char *str);

// Free memory and zero fields
void VString_free(VString *str);

// Add a C-string to the end
int VString_push_str(VString *str, const char *suffix);

// Add a single character
int VString_push_char(VString *str, char c);

// Deletes the last character
int VString_pop(VString *str);

// Get character by index
char VString_get(VString *str, int idx);

// Pushes a character at the specified index
int VString_insert(VString *str, char ch, int idx);

// Clears the content (does not free memory)
void VString_clear_fast(VString *str);

// Clears the content (frees memory)
void VString_clear_full(VString *str);

// Returns the length of the string
size_t VString_len(const VString *str);

// Checks if the string is empty
bool VString_is_empty(const VString *str);

// Convert to C-string (for printf, exec, etc.)
const char *VString_cstr(const VString *str);

// Compare with another VString
bool VString_cmp(const VString *a, const VString *b);

// Compares with a C-string
bool VString_cmp_cstr(const VString *a, const char *b);

// Find a character, returns index or -1
size_t VString_find_char(const VString *str, char ch);

// Find a substring (C-string), returns index or -1
size_t VString_find_str(const VString *str, const char *substr);

// Removes the suffix if it exists (e.g., ".service")
bool VString_strip_suffix(VString *str, const char *suffix);

// Removes the prefix (e.g., "/etc/void/")
bool VString_strip_prefix(VString *str, const char *prefix);

// Adds a formatted string: VString_push_fmt(&s, "pid=%d", pid);
int VString_push_fmt(VString *str, const char *fmt, ...);

// Replaces the first occurrence of old with new
int VString_replace_first(VString *str, const char *old, const char *new_str);

// Converts to a character and inserts it into the string
int VString_push_int(VString *str, int value);

// Converts to a character and inserts it into the string
int VString_push_float(VString *str, float value);

// Checks if the string starts with a prefix
bool VString_starts_with(const VString *str, const char *prefix);

// Check if the string ends with a suffix
bool VString_ends_with(const VString *str, const char *suffix);

// Removes spaces/tabs from the beginning
void VString_trim_start(VString *str);

// Removes spaces/tabs from the end
void VString_trim_end(VString *str);    

// Removes spaces/tabs from both ends
void VString_trim(VString *str);

// Full copy of the string
VString VString_clone(const VString *str);

// Checks if the string is NULL
bool VString_valid(const VString *str);

// Creates a string from a part of a C-string
VString VString_from_n(const char *str, size_t len);

// Truncates the string to the specified length
void VString_truncate(VString *str, size_t new_len);

// Get substring
VString VString_substr(const VString *str, size_t start, size_t len);

// Converts to lowercase
void VString_to_lowercase(VString *str);

// Converts to uppercase
void VString_to_uppercase(VString *str);

// Check if the string contains only digits
bool VString_is_digit(const VString *str);        

// Checks if the string contains only hexadecimal characters
bool VString_is_hex(const VString *str);

// Replaces all occurrences of old with new
int VString_replace_all(VString *str, const char *old, const char *new_str);

#endif