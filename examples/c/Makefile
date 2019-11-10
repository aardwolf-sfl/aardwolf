LLVM_PASSES_PATH = ../../frontends/llvm/build/lib
RUNTIME_PATH = ../../runtime/build
OUTPUT_PATH = bin/aardwolf

test: src/* tests/*
	@mkdir -p bin
	@clang -g -o bin/test tests/test.c src/*.c ${RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./bin/test

aardwolf: src/* tests/*
	@rm -rf ${OUTPUT_PATH}
	@mkdir -p ${OUTPUT_PATH}
	@export LLVM_PASSES_PATH=${LLVM_PASSES_PATH} ; export RUNTIME_PATH=${RUNTIME_PATH} ; export AARDWOLF_DATA_DEST=${OUTPUT_PATH} ; bash use_aardwolf.sh
	@export AARDWOLF_DATA_DEST=${OUTPUT_PATH} ; ./bin/test