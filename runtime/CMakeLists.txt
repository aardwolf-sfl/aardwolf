cmake_minimum_required(VERSION 3.13)
project(aardwolf_runtime)

# C99
set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED on)

message(STATUS "Build type: ${CMAKE_BUILD_TYPE} with compiler: ${CMAKE_C_COMPILER}")

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra -pedantic")

if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_definitions(-DDEBUG_ENABLED)
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Werror")
elseif (CMAKE_BUILD_TYPE STREQUAL "Release")
endif ()

add_library(aardwolf_runtime STATIC runtime.c)
