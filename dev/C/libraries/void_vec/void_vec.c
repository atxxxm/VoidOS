#include "void_vec.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>
#include <stdarg.h>


// initial capacity
#define VEC_INITIAL_CAP 16

void vvec_init(VVec* vec, size_t elem_size) {
    if (vec == NULL) return;

    // init
    vec->len = 0;
    vec->elem_size = elem_size;
    vec->cap = VEC_INITIAL_CAP;

    // allocate
    vec->data = malloc(elem_size * vec->cap);

    // if malloc failed, set to 0 and return
    if (vec->data == NULL) {
        vec->cap = 0;
        vec->len = 0;
        return;
    }
}

void vvec_push(VVec* vec, const void* elem) {
    // if vec is null or elem is null, return
    if (vec == NULL || elem == NULL) return;

    // if vec is full, resize
    if (vec->len >= vec->cap) {
        size_t new_cap = {0};

        if (vec->cap == 0) {
            new_cap = 1;
        } else {
            new_cap = vec->cap * 2;
        }

        // realloc data
        void *new_data = realloc(vec->data, new_cap * vec->elem_size);

        // if realloc failed, return
        if (new_data == NULL) return;

        // update cap
        vec->data = new_data;
        vec->cap = new_cap;
    }

    // copy elem to data
    memcpy((char*)vec->data + vec->len * vec->elem_size, elem, vec->elem_size);

    // increment len
    vec->len++;
}

void vvec_pop(VVec* vec) {
    // if vec is null or empty, return
    if (vec == NULL || vec->data == NULL || vec->len == 0) return;

    // decrement len
    vec->len--;
}

bool vvec_get(const VVec* vec, size_t index, void* out_elem) {
    // if vec is null or empty, return
    if (vec == NULL || vec->data == NULL || index >= vec->len || out_elem == NULL) {
        return false;
    }

    // copy elem to out_elem
    memcpy(out_elem, (char*)vec->data + index + vec->elem_size, vec->elem_size);
    return true;
}

bool vvec_set(VVec* vec, size_t index, const void* elem) {
    if (vec == NULL || elem == NULL || index >= vec->len) {
        return false;
    }

    // copy elem to data
    memcpy((char*)vec->data + index * vec->elem_size, elem, vec->elem_size);
    return true;
}

void vvec_clear(VVec* vec) {
    vec->len = 0;
}

void vvec_free(VVec* vec) {
    // if vec is null, return
    if (vec != NULL) {
        free(vec->data);
        vec->data = NULL;
        vec->len = 0;
        vec->cap = 0;
        vec->elem_size = 0;
    }
}

void* vvec_at(VVec* vec, size_t index) {
    // if vec is null or index is out of bounds, return NULL
    if (index >= vec->len) return NULL;

    return (char*)vec->data + index * vec->elem_size;
}
