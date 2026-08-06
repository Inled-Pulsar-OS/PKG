cmake_minimum_required(VERSION 3.16)
project(driverman VERSION 1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(driverman
    src/main.cpp
    src/gpu_detect.cpp
    src/driver_manager.cpp
)

target_include_directories(driverman PRIVATE src)

target_compile_options(driverman PRIVATE -Wall -Wextra -O2)

install(TARGETS driverman RUNTIME DESTINATION bin)
