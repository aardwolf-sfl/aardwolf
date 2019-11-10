test: src/* tests/*
	@mkdir -p bin
	@clang -g -o bin/test tests/test.c src/*.c ${AARDWOLF_RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./bin/test

aardwolf: src/* tests/*
	./aardwolf_llvm -o bin -s "src/*.c" -t tests/test.c
	@echo
	@cat bin/aardwolf/!aardwolf.test