cmake_minimum_required(VERSION 3.10)

project(demons)

set(CMAKE_C_STANDARD 11)  
set(CMAKE_C_STANDARD_REQUIRED ON)

set(CMAKE_EXE_LINKER_FLAGS "-static")

add_executable(syslog syslog.c)
add_executable(crond crond.c)