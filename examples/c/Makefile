OUTPUT_DIRECTORY = bin

test: src/* tests/*
	@mkdir -p ${OUTPUT_DIRECTORY}
	@clang -g -o ${OUTPUT_DIRECTORY}/test tests/test.c src/*.c ${AARDWOLF_RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./${OUTPUT_DIRECTORY}/test

aardwolf: src/* tests/*
	@rm -rf ${OUTPUT_DIRECTORY}
	@mkdir -p ${OUTPUT_DIRECTORY}
	@cd ${OUTPUT_DIRECTORY} && clang -g -c -emit-llvm ../src/*.c
	@llvm-link -o ${OUTPUT_DIRECTORY}/\!linked.bc ${OUTPUT_DIRECTORY}/*.bc
	@./aardwolf_llvm -o ${OUTPUT_DIRECTORY} ${OUTPUT_DIRECTORY}/\!linked.bc
	@clang -g -o ${OUTPUT_DIRECTORY}/\!run tests/test.c ${OUTPUT_DIRECTORY}/\!instrumented.bc ${AARDWOLF_RUNTIME_PATH}/libaardwolf_runtime.a
	@export AARDWOLF_DATA_DEST=${OUTPUT_DIRECTORY} && ./${OUTPUT_DIRECTORY}/\!run
