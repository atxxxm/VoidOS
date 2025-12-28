#ifndef VOID_VEC_H
#define VOID_VEC_H

#include <stddef.h>
#include <stdbool.h>

typedef struct {
    void* data;
    size_t len;
    size_t cap;
    size_t elem_size; // Size of one element in bytes
} VVec;

// Initialize a new vector
void vvec_init(VVec* vec, size_t elem_size);

// Add an element to the vector
void vvec_push(VVec* vec, const void* elem);

// Delete the last element from the vector
void vvec_pop(VVec* vec);

// Get the element at the specified index
bool vvec_get(const VVec* vec, size_t index, void* out_elem);

// Set the element at the specified index
bool vvec_set(VVec* vec, size_t index, const void* elem);

// Clear the vector, but not free the memory
void vvec_clear(VVec* vec);

// Free memory
void vvec_free(VVec* vec);

// Returns a pointer to an element
void* vvec_at(VVec* vec, size_t index);

#endif