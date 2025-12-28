cmake_minimum_required(VERSION 3.10)
project(VoidString)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)


add_library(void_string STATIC void_string.c)

target_include_directories(void_string PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>
    $<INSTALL_INTERFACE:include>
)