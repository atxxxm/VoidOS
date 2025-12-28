cmake_minimum_required(VERSION 3.10)

project(DemonController)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(CMAKE_EXE_LINKER_FLAGS "-static")

include_directories(${CMAKE_SOURCE_DIR}/library/include)

set(VOID_VEC_LIB ${CMAKE_SOURCE_DIR}/library/lib/libvoid_vec.a)
set(VOID_STRING_LIB ${CMAKE_SOURCE_DIR}/library/lib/libvoid_string.a)
set(VOID_FILE_LIB ${CMAKE_SOURCE_DIR}/library/lib/libvoid_file.a)

if(NOT EXISTS ${VOID_VEC_LIB})
    message(FATAL_ERROR "libvoid_vec.a not found at ${VOID_VEC_LIB}")
endif()

if(NOT EXISTS ${VOID_STRING_LIB})
    message(FATAL_ERROR "libvoid_string.a not found at ${VOID_STRING_LIB}")
endif()

if(NOT EXISTS ${VOID_FILE_LIB})
    message(FATAL_ERROR "libvoid_file.a not found at ${VOID_FILE_LIB}")
endif()


add_executable(demon-controller main.c)

target_link_libraries(
    demon-controller
    ${VOID_VEC_LIB}
    ${VOID_FILE_LIB}
    ${VOID_STRING_LIB}
)