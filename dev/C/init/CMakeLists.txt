cmake_minimum_required(VERSION 3.10)

project(Init)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(CMAKE_EXE_LINKER_FLAG "-static")

add_executable(init init.c)