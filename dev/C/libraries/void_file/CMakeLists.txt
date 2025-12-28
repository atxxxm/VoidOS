cmake_minimum_required(VERSION 3.10)
project(VoidFile)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

include_directories(${CMAKE_SOURCE_DIR}/../void_string)
set(VOID_STRING_LIB ${CMAKE_SOURCE_DIR}/../lib/libvoid_string.a)

if(NOT EXISTS ${VOID_STRING_LIB})
    message(FATAL_ERROR "void_string library not found at ${VOID_STRING_LIB}")
endif()

add_library(void_file STATIC void_file.c)

target_link_libraries(void_file ${VOID_STRING_LIB})



