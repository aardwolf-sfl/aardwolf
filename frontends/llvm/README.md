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
opt -load-pass-plugin path/to/libAardwolfLLVM.so -passes=aardwolf-static-data,aardwolf-dynamic-data source.bc > instrumented.bc
clang -o instrumented instrumented.bc path/to/libaardwolf_runtime.a
./instrumented
```

## Note on coding style

We try to comply with LLVM coding style, even when it is different from usual C++ code style.
