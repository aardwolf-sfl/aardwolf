# Aardwolf Runtime

## Usage

**Compilation:**

```
mkdir build && cd build
cmake ..
make
```

## Description

* `libaardwolf_runtime.a` - Full runtime which should be used in majority of use cases. It should be bundled with the test runner code which should only call `aardwolf_write_external` and let instrumented code output the rest.
* `libaardwolf_runtime_bare.a` - Runtime which does not write the file header when trace file is created. This is used when the trace is built sequentially by calling external programs that call `aardwolf_write_external` (but every time they open a new file descriptor).
* `libaardwolf_runtime_noop.a` - This version of runtime does nothing and should be used during testing without Aardwolf if linking some runtime is necessary not to get a linking error.
* `aardwolf_external` - A trivial program that implements use case of `libaardwolf_runtime_bare.a`. In your test script, in the very beginning execute it without any arguments and later execute it with the test name as its first argument.
