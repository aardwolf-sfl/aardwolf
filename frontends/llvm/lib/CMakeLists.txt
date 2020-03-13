set(PLUGIN_NAME AardwolfLLVM)

add_library(${PLUGIN_NAME} SHARED Registration.cpp StaticData.cpp DynamicData.cpp StatementDetection.cpp Statement.cpp StatementRepository.cpp Tools.cpp)
target_include_directories(${PLUGIN_NAME} PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/../include")
