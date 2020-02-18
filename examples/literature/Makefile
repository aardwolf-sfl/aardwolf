OUTPUT_DIRECTORY = bin

ppdg: src/* tests/*
	@mkdir -p ${OUTPUT_DIRECTORY}
	@clang -g -o ${OUTPUT_DIRECTORY}/ppdg tests/ppdg.c src/ppdg.c ${AARDWOLF_RUNTIME_PATH}/libaardwolf_runtime_noop.a
	@./${OUTPUT_DIRECTORY}/ppdg
