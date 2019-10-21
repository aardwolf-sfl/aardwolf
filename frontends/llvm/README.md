# LLVM Frontend

## Usage

**Compilation:**

```
mkdir build && cd build
cmake ..
make
```

**Running:**

```
clang -c -emit-llvm -g -o source.bc source.c
opt -load path/to/libLLVMStatementDetection.so -load path/to/libLLVMStaticData.so -load path/to/libLLVMExecutionTrace.so -aard-static-data -aard-exec-trace source.bc > instrumented.bc
clang -o instrumented instrumented.bc path/to/libaardwolf_runtime.a
./instrumented
```

## Note on coding style

We try to comply with LLVM coding style, even when it is different from usual C++ code style.
