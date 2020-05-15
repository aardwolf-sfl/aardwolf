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

add_library(aardwolf_runtime SHARED runtime.c)
add_library(aardwolf_runtime_bare SHARED runtime.c)
add_library(aardwolf_runtime_noop SHARED runtime.c)

set_target_properties(aardwolf_runtime_bare PROPERTIES COMPILE_FLAGS "-DNO_HEADER")
set_target_properties(aardwolf_runtime_noop PROPERTIES COMPILE_FLAGS "-DNO_DATA -Wno-unused-parameter")

add_library(aardwolf_runtime_static STATIC runtime.c)
add_library(aardwolf_runtime_bare_static STATIC runtime.c)
add_library(aardwolf_runtime_noop_static STATIC runtime.c)

set_target_properties(aardwolf_runtime_bare_static PROPERTIES COMPILE_FLAGS "-DNO_HEADER")
set_target_properties(aardwolf_runtime_noop_static PROPERTIES COMPILE_FLAGS "-DNO_DATA -Wno-unused-parameter")
set_target_properties(aardwolf_runtime_static PROPERTIES OUTPUT_NAME aardwolf_runtime)
set_target_properties(aardwolf_runtime_bare_static PROPERTIES OUTPUT_NAME aardwolf_runtime_bare)
set_target_properties(aardwolf_runtime_noop_static PROPERTIES OUTPUT_NAME aardwolf_runtime_noop)

add_executable(aardwolf_external aardwolf_external.c)
target_link_libraries(aardwolf_external aardwolf_runtime_bare_static)
