cmake_minimum_required(VERSION 3.13)
project(aardwolf_llvm)

# C++17
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED on)

message(STATUS "Build type: ${CMAKE_BUILD_TYPE} with compiler: ${CMAKE_CXX_COMPILER}")

find_package(LLVM REQUIRED CONFIG)

message(STATUS "LLVM ${LLVM_PACKAGE_VERSION}")

if (LLVM_SRC_PATH)
    include_directories(${LLVM_SRC_PATH}/include)
    message(STATUS "LLVM headers: ${LLVM_SRC_PATH}/include")
else()
    include_directories(${LLVM_INCLUDE_DIRS})
    message(STATUS "LLVM headers: ${LLVM_INCLUDE_DIRS}")
endif()

if (LLVM_BUILD_PATH)
    link_directories(${LLVM_BUILD_PATH}/lib)

    # llvm-config.h
    include_directories("${LLVM_BUILD_PATH}/include")
    message(STATUS "LLVM libraries: ${LLVM_BUILD_PATH}/lib")
else()
    link_directories(${LLVM_LIBRARY_DIRS})
    message(STATUS "LLVM libraries: ${LLVM_LIBRARY_DIRS}")
endif(LLVM_BUILD_PATH)

add_definitions(${LLVM_DEFINITIONS})

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -pedantic")

if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_definitions(-DDEBUG_ENABLED)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Werror")
elseif (CMAKE_BUILD_TYPE STREQUAL "Release")
endif ()

add_subdirectory(lib)
