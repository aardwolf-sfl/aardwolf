cmake_minimum_required(VERSION 3.13)
project(aardwolf_llvm)

# Use C++17.
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED on)

find_package(LLVM 9.0.0 REQUIRED CONFIG)

# Write details about found LLVM.
message(STATUS "LLVM ${LLVM_PACKAGE_VERSION}")
message("  Definitions: ${LLVM_DEFINITIONS}
  Includes:    ${LLVM_INCLUDE_DIRS}
  Libraries:   ${LLVM_LIBRARY_DIRS}
  Targets:     ${LLVM_TARGETS_TO_BUILD}"
)
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Compiler:   ${CMAKE_CXX_COMPILER}")

# Set the LLVM header and library paths.
include_directories(${LLVM_INCLUDE_DIRS})
link_directories(${LLVM_LIBRARY_DIRS})
add_definitions(${LLVM_DEFINITIONS})

# Compiler flags.
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -pedantic")

# Compiler flags depending on build type.
if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_definitions(-DDEBUG_ENABLED)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Werror")
elseif (CMAKE_BUILD_TYPE STREQUAL "Release")
endif ()

# LLVM is normally built without RTTI. Be consistent with that.
if(NOT LLVM_ENABLE_RTTI)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti")
endif()

# Set the build directories.
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${PROJECT_BINARY_DIR}/bin")
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${PROJECT_BINARY_DIR}/lib")
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY "${PROJECT_BINARY_DIR}/lib")

# Add sub-targets.
add_subdirectory(lib)
add_subdirectory(cli)
