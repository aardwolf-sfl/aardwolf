# Go to bin directory in order to generate bitcode files there
cd bin

# Prepare source code bitcode files
clang -c -emit-llvm -g ../src/*.c

# Go back
cd ..

# Instrument them
for bitcode_file in bin/*.bc
do
    opt -load $LLVM_PASSES_PATH/libLLVMStatementDetection.so -load $LLVM_PASSES_PATH/libLLVMStaticData.so -load $LLVM_PASSES_PATH/libLLVMExecutionTrace.so -aard-static-data -aard-exec-trace $bitcode_file > $(echo $bitcode_file | sed -e 's/.bc/_instr.bc/g')
done

# Compile them together with test runner
clang -o bin/test tests/test.c bin/*_instr.bc $RUNTIME_PATH/libaardwolf_runtime.a

# Clean temporaries
rm -rf bin/*.bc
