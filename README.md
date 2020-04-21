# Aardwolf: A Modular Tool for Software Fault Localization

## Usage

```sh
# Clone this repository
git clone https://gitlab.fit.cvut.cz/nevyhpet/master-thesis-code aardwolf
# Go to the repository
cd aardwolf
# Run the installation script (note: if you want to install Python frontend system-wide, use `--system-python` flag)
python aardwolf-install.py
# You can find all (successfully) built components in $HOME/.aardwolf directory
```

The installation script automatically checks for existence of required commands
as well as their versions.


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
