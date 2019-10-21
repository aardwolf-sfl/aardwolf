LLVM_PASSES_PATH = ../../frontends/llvm/build/lib
RUNTIME_PATH = ../../runtime/build

test: src/* tests/*
	@mkdir -p bin
	@clang -o bin/test tests/test.c src/*.c ${RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./bin/test

aardwolf: src/* tests/*
	@mkdir -p bin/aardwolf
	@rm -rf bin/aardwolf/*
	@export LLVM_PASSES_PATH=${LLVM_PASSES_PATH} ; export RUNTIME_PATH=${RUNTIME_PATH} ; bash use_aardwolf.sh
	@./bin/test
	@mv aardwolf.* bin/aardwolf