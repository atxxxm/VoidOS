#include "void_file.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>



VFile* VFile_open(const char* path, const char* mode) {
    // Open the file
    FILE* fp = fopen(path, mode);

    // Check for errors
    if (!fp) {
        perror("File open error");;
        return NULL;
    }

    // Allocate memory for the VFile wrapper
    VFile* vf = malloc(sizeof(VFile));

    // Check for allocation failure
    if (vf == NULL) {
        fclose(fp);
        perror("Memory allocation error");
        return NULL;
    }

    // Initialize the VFile wrapper
    vf->inner = fp;
    vf->path = strdup(path);

    // Check for allocation failure
    if (vf->path == NULL) {
        free(vf);
        fclose(fp);
        perror("Memory allocation error");
        return NULL;
    }

    // Set ownership flag
    vf->owns_fp = 1;

    return vf;
}

VFile* VFile_from_fp(FILE* fp, int take_ownership) {
    // Check for null FILE*
    if (!fp) {
        perror("File opening error");
        return NULL;
    }

    // Allocate memory for the VFile wrapper
    VFile* vf = malloc(sizeof(VFile));

    // Check for allocation failure
    if (vf == NULL) {
        perror("Error allocating memory for the vfile structure!");
        return NULL;
    }

    // Initialize the VFile wrapper
    vf->inner = fp;
    vf->path = NULL;
    vf->owns_fp = take_ownership;

    return vf;
}

void VFile_close(VFile* vf) {
    // Check for null VFile
    if (vf == NULL) {
        perror("The structure vfIle is NULL");
        return;
    }

    // Close the file
    if (vf->inner && vf->owns_fp) {
        fclose(vf->inner);
    }

    // Free memory
    free(vf->path);
    free(vf);
}

size_t VFile_read_line(VFile* vf, VString* vs) {
    if (vf == NULL || vs == NULL) return 0;

    int c;

    // Read line
    while((c = fgetc(vf->inner)) != EOF) {
        if (c == '\n') return 1;

        // push char
        VString_push_fmt(vs, "%c", c);
    }

    return (VString_len(vs) > 0) ? 1 : 0;
}

size_t VFile_read(VFile* vf, void* buffer, size_t size) {
    if (vf == NULL || buffer == NULL || vf->inner == NULL) return 0;

    if (ferror(vf->inner)) return 0;

    return fread(buffer, 1, size, vf->inner);
}

int VFile_read_char(VFile* vf) {
    if (vf->inner == NULL|| vf == NULL) {
        return EOF;
    }

    return fgetc(vf->inner);
}

size_t VFile_read_all(VFile* vf, VString* vs) {
    if (vf == NULL || vf->inner == NULL || vs == NULL) return 0;

    int c;
    size_t count = 0;

    // Read all
    while ((c = fgetc(vf->inner)) != EOF) 
    {
        VString_push_char(vs, (char)c);
        count++;
    }

    // Check for errors
    if (ferror(vf->inner)) {
        clearerr(vf->inner);
        return 0;
    }

    return count;
}

size_t VFile_write(VFile* vf, const void* data, size_t size) {
    if (vf == NULL || vf->inner == NULL || data == NULL) return 0;

    return fwrite(data, 1, size, vf->inner);
}

size_t VFile_write_str(VFile* vf, VString *vs) {
    if (vf == NULL || vf->inner == NULL || vs == NULL) return 0;

    // Get the string
    const char *str = VString_cstr(vs);

    // Check for null
    if (str == NULL) return 0;

    return fwrite(str, 1, vs->len, vf->inner);
}

size_t VFile_write_line(VFile* vf,  const char* line) {
    if (vf == NULL || vf->inner == NULL || line == NULL) return 0;

    // Write the line
    size_t len = strlen(line);
    size_t written = fwrite(line, 1, len, vf->inner);

    // Check length
    if (written != len) return written;

    // Write newline
    if (fwrite("\n", 1, 1, vf->inner) != 1) {
        return len;
    }

    return len + 1;
}

int VFile_printf(VFile* vf, const char* format, ...) {
    if (vf == NULL || vf->inner == NULL || format == NULL) return 0;

    // Use va_list
    va_list args;
    va_start(args, format);

    // Write the line 
    int result = vfprintf(vf->inner, format, args);
    va_end(args);

    return result;
}

long VFile_tell(VFile* vf) {
    if (vf == NULL || vf->inner == NULL) return -1;

    return ftell(vf->inner);
}

int VFile_seek(VFile* vf, long offset, int whence) {
    if (vf == NULL || vf->inner == NULL) return -1;

    return fseek(vf->inner, offset, whence);
}

bool VFile_eof(VFile* vf) {
    if (vf == NULL || vf->inner == NULL) return false;

    return feof(vf->inner) != 0;
}

bool VFile_error(VFile* vf) {
    if (vf == NULL || vf->inner == NULL) return false;

    return ferror(vf->inner) != 0;
}

void VFile_clearerr(VFile* vf) {
    if (vf == NULL || vf->inner == NULL) return;

    clearerr(vf->inner);
}
