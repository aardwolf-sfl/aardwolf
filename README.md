# Aardwolf: A Modular Tool for Software Fault Localization

**In alpha stage of development. Only for experimental purposes!**

## Installation

* Clone this repository and go to its directory
* Run `python aardwolf-install.py`

The installation script will automatically check for dependencies and by default copies compiled artifacts into `$HOME/.aardwolf` directory.
Type `python aardwolf-install.py --help` for more information.

## Try

### C

**example/c**

```sh
# Apply a bug
git apply bug1.patch # generally bugN.patch
# Run Aardwolf
~/.aardwolf/aardwolf
```

### Python

**example/python**

```sh
# Apply a bug
git apply bug1.patch # generally bugN.patch
# Run Aardwolf
poetry run ~/.aardwolf/aardwolf
```

## Dev Guide

The structure of the repository is as follows:

* [`core`](core) -- Rust's project containing Aardwolf core (driver, data loading, plugins, user interfaces, etc.).
* [`examples`](examples) -- A collection of toy projects that utilize Aardwolf to show how it can be used.
* [`frontends`](frontends) -- Directory for official frontends.
    * [`llvm`](frontends/llvm) -- LLVM frontend for C language.
    * [`python`](frontends/python) -- Python frontend for Python language.
* [`runtime`](runtime) -- C runtime used by LLVM frontend that can be considered as the official implementation.
* [`tools`](tools) -- Small Python package that implements some tools aiding the development of Aardwolf ecosystem.
* [`aardwolf-install.py`](aardwolf-install.py) -- Installation script that compiles all te necessary components and copies them into specified directory.