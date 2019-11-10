# C99
set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED on)

message(STATUS "Build type: ${CMAKE_BUILD_TYPE} with compiler: ${CMAKE_C_COMPILER}")

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra -pedantic")
SET(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -lm")

if (CMAKE_BUILD_TYPE STREQUAL "Debug")
add_definitions(-DDEBUG_ENABLED)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Werror")
elseif (CMAKE_BUILD_TYPE STREQUAL "Release")
endif ()

add_executable(aardwolf_llvm cli.c argtable3/argtable3.c utils-linux.c)
