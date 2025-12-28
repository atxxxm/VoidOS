#ifndef VOID_FILE_H
#define VOID_FILE_H

#include <stdio.h>
#include <stdbool.h>
#include "../void_string/void_string.h"

typedef struct {
    FILE* inner;
    char* path;
    int owns_fp;
} VFile;

// Opens a file and returns a VFile wrapper
VFile* VFile_open(const char* path, const char* mode);

// Creates a VFile wrapper from an existing FILE*
VFile* VFile_from_fp(FILE* fp, int take_ownership);

// Closes the file and frees the VFile wrapper
void VFile_close(VFile* vf);

// Reads a line from the file
size_t VFile_read_line(VFile* vf, VString* vs);

// Reads size bytes into buffer
size_t VFile_read(VFile* vf, void* buffer, size_t size);

// Reads one character (return EOF on error/EOF)
int VFile_read_char(VFile* vf);

// Reads the entire file into a string (assumed to be a text file; null-terminates)
size_t VFile_read_all(VFile* vf, VString* vs);

// Writes size bytes
size_t VFile_write(VFile* vf, const void* data, size_t size);

// Write a string (without null terminator)
size_t VFile_write_str(VFile* vf, VString *vs);

// Write a line (with \n)
size_t VFile_write_line(VFile* vf,  const char* line);

// Write with arguments
int VFile_printf(VFile* vf, const char* format, ...);

// Returns the current position
long VFile_tell(VFile* vf);

// Moves the file pointer
int VFile_seek(VFile* vf, long offset, int whence);

// Checks if the end of file has been reached
bool VFile_eof(VFile* vf);

// Checks if an error has occurred
bool VFile_error(VFile* vf);

// Resets the error flag
void VFile_clearerr(VFile* vf);

#endif
