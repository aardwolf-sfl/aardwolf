add_executable(aardwolf_llvm Main.cpp)

llvm_map_components_to_libnames(REQ_LLVM_LIBRARIES ${LLVM_TARGETS_TO_BUILD}
    passes
)

target_link_libraries(aardwolf_llvm AardwolfLLVM ${REQ_LLVM_LIBRARIES})
