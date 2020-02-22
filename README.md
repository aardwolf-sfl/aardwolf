# Aardwolf: A Modular Tool for Software Fault Localization

## Requirements

*Listed versions are the versions on which this project was tested on, but others could work as well.*

* [CMake](https://cmake.org/) v3.13
* [LLVM](https://llvm.org/) v9.0
* [Rust](https://www.rust-lang.org/) v1.41

## Usage

### Compile LLVM frontend

```sh
# In frontends/llvm directory
# Create build directory
mkdir build && cd build
# Initialize build configuration
cmake ..
# Compile
make
```

### Compile runtime

```sh
# In runtime directory
# Create build directory
mkdir build && cd build
# Initialize build configuration
cmake ..
# Compile
make
```

### Compile Aardwolf

```sh
# In core directory
# Compile
cargo build
```

### Try Aardwolf on an example

```sh
# In examples/c directory
# Apply a bug
git apply bug1.patch # or bug2.patch
# Run Aardwolf
../../core/target/debug/aardwolf --runtime ../../runtime/build/libaardwolf_runtime.a --frontend ../../frontends/llvm/build/bin/aardwolf_llvm
```
