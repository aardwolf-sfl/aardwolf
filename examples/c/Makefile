OUTPUT_DIRECTORY = bin

test: src/* tests/*
	@mkdir -p ${OUTPUT_DIRECTORY}
	@clang -g -o ${OUTPUT_DIRECTORY}/test tests/test.c src/*.c ${AARDWOLF_RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./${OUTPUT_DIRECTORY}/test
