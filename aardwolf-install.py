import sys
import os
import subprocess
import shutil
import glob
import time
import re
import argparse
from itertools import chain


ROOT_DIR = os.path.realpath(os.path.dirname(__file__))
DEFAULT_DEST_DIR = os.path.join(os.path.expanduser('~'), '.aardwolf')

LOG_INDENT = 0


def main():
    parser = argparse.ArgumentParser(description='Aardwolf installer')
    parser.add_argument('--destination', '-d', default=DEFAULT_DEST_DIR)
    parser.add_argument('--system-python', action='store_true')
    parser.add_argument('--force', '-f', action='store_true')
    parser.add_argument('--debug', action='store_true')

    args = parser.parse_args()

    if args.force:
        warn('Your are running the script with --force flag, required versions of tools are not checked')

    prepare_dest(args)

    deps = check_deps(args)

    if 'rust' in deps:
        install_core(args)

    if 'llvm' in deps:
        install_llvm(args)
        install_runtime(args)

    if 'python' in deps:
        install_python(args)

    need_cmd(
        'aardwolf',
        f'Command `aardwolf` is not executable, you may want to add {args.destination} to your PATH',
        level='warn')


def indent():
    return ''.join(['  ' for _ in range(LOG_INDENT)])


def inc_indent():
    global LOG_INDENT
    LOG_INDENT += 1


def dec_indent():
    global LOG_INDENT
    LOG_INDENT -= 1


def info(message):
    print(f'[aardwolf  INFO]:{indent()} {message}')


def warn(message):
    print(f'[aardwolf  WARN]:{indent()} {message}')


def error(message, exit_after=True):
    print(f'[aardwolf ERROR]:{indent()} {message}', file=sys.stderr)

    if exit_after:
        exit(1)


def need_cmd(cmd, message=None, level='error'):
    if isinstance(cmd, list):
        return all([need_cmd(c) for c in cmd])

    if shutil.which(cmd) is None:
        if message is None:
            message = f'Command `{cmd}` is required for installation'

        if level == 'error':
            error(message)
        elif level == 'warn':
            warn(message)

        return False
    else:
        return True


def run_cmd(cmd, args=None, cwd=None):
    if args is None:
        args = []

    subprocess_args = [cmd]
    subprocess_args.extend(args)

    try:
        return subprocess.run(subprocess_args, capture_output=True, cwd=cwd, check=True, encoding='utf-8').stdout
    except subprocess.CalledProcessError as ex:
        command = ' '.join(subprocess_args)
        error(f'Command {command} failed for this reason:', exit_after=False)
        print(file=sys.stderr)
        print(ex.stderr, file=sys.stderr)
        print(file=sys.stderr)
        exit(1)


def prepare_dest(args):
    if os.path.isdir(args.destination):
        info(f'Aardwolf already installed at "{args.destination}", upgrading')
        shutil.rmtree(args.destination)

    os.makedirs(args.destination)


def check_deps(args):
    RUST_MINOR = 43
    LLVM_MAJOR = 9
    PYTHON_MAJOR = 3
    PYTHON_MINOR = 8

    semver_pattern = re.compile(r'(\d+)\.(\d+)\.(\d+)')
    satisfied = set()

    info('Checking dependencies')

    if need_cmd(['rustc', 'cargo'], 'Rust not found, Aardwolf core cannot be installed'):
        rust_version = run_cmd('rustc', ['--version'])
        rust_version_match = semver_pattern.search(rust_version)

        if rust_version_match is None:
            error('Retrieving LLVM version failed')

        rust_minor = int(rust_version_match.group(2))

        if rust_minor >= RUST_MINOR:
            info('Rust dependency satisfied')
            satisfied.add('rust')
        else:
            required = f'1.{RUST_MINOR}'
            warn(f'Rust in unsupported version ({required} is required), ')

            if args.force:
                satisfied.add('rust')
            else:
                error('Aardwolf core cannot be installed')

    if need_cmd('llvm-config', 'LLVM not found, LLVM frontend will not be installed', level='warn'):
        llvm_version = run_cmd('llvm-config', ['--version'])
        llvm_version_match = semver_pattern.search(llvm_version)

        if llvm_version_match is None:
            error('Retrieving LLVM version failed')

        llvm_major = int(llvm_version_match.group(1))

        if llvm_major >= LLVM_MAJOR:
            info('LLVM dependency satisfied')
            satisfied.add('llvm')
        else:
            required = LLVM_MAJOR
            warn(f'LLVM in unsupported version ({required} is required)')

            if args.force:
                satisfied.add('llvm')
            else:
                error('LLVM frontend will not be installed')

    if need_cmd(['python3', 'pip3'], 'Python not found, Python frontend will not be installed', level='warn'):
        python_version = run_cmd('python3', ['--version'])
        python_version_match = semver_pattern.search(python_version)

        if python_version_match is None:
            error('Retrieving Python version failed')

        python_major = int(python_version_match.group(1))
        python_minor = int(python_version_match.group(2))

        if python_major >= PYTHON_MAJOR and python_minor >= PYTHON_MINOR:
            info('Python dependency satisfied')
            satisfied.add('python')
        else:
            required = f'{PYTHON_MAJOR}.{PYTHON_MINOR}'
            warn(f'Python in unsupported version ({required} is required)')

            if args.force:
                satisfied.add('python')
            else:
                error('Python frontend will not be installed')

    return satisfied


def install_core(args):
    info('Installing Aardwolf core')
    inc_indent()

    need_cmd(['cargo'])

    mode = 'release' if not args.debug else 'debug'

    source_dir = os.path.join(ROOT_DIR, 'core')
    build_dir = os.path.join(source_dir, 'target', mode)

    info('Compile')
    cmd_args = ['build']
    if not args.debug:
        cmd_args.append('--release')
    run_cmd('cargo', cmd_args, cwd=source_dir)

    info('Install')
    shutil.copy(os.path.join(build_dir, 'aardwolf'),
                os.path.join(args.destination, 'aardwolf'))

    dec_indent()
    info('Aardwolf core was successfully installed')


def install_llvm(args):
    info('Installing LLVM frontend')
    inc_indent()

    need_cmd(['cmake', 'make'])

    mode_dir = 'release' if not args.debug else 'debug'
    mode = 'Release' if not args.debug else 'Debug'

    source_dir = os.path.join(ROOT_DIR, 'frontends', 'llvm')
    build_dir = os.path.join(source_dir, 'build', mode_dir)

    info('Compile')
    os.makedirs(build_dir, exist_ok=True)

    run_cmd('cmake', [f'-DCMAKE_BUILD_TYPE={mode}', source_dir], cwd=build_dir)
    run_cmd('make', cwd=build_dir)

    info('Install')
    shutil.copy(os.path.join(build_dir, 'lib', 'libAardwolfLLVM.so'),
                os.path.join(args.destination, 'libAardwolfLLVM.so'))
    shutil.copy(os.path.join(build_dir, 'bin', 'aardwolf_llvm'),
                os.path.join(args.destination, 'aardwolf_llvm'))

    dec_indent()
    info('LLVM frontend was successfully installed')


def install_runtime(args):
    info('Installing C runtime')
    inc_indent()

    need_cmd(['cmake', 'make'])

    mode_dir = 'release' if not args.debug else 'debug'
    mode = 'Release' if not args.debug else 'Debug'

    source_dir = os.path.join(ROOT_DIR, 'runtime')
    build_dir = os.path.join(source_dir, 'build', mode_dir)

    info('Compile')
    os.makedirs(build_dir, exist_ok=True)

    run_cmd('cmake', [f'-DCMAKE_BUILD_TYPE={mode}', source_dir], cwd=build_dir)
    run_cmd('make', cwd=build_dir)

    info('Install')
    shutil.copy(os.path.join(build_dir, 'aardwolf_external'),
                os.path.join(args.destination, 'aardwolf_external'))

    libfiles = chain(*[glob.glob(os.path.join(build_dir, f'*.{ext}'))
                       for ext in ['so', 'a']])

    for libfile in libfiles:
        libfile = os.path.basename(libfile)

        shutil.copy(os.path.join(build_dir, libfile),
                    os.path.join(args.destination, libfile))

    dec_indent()
    info('C runtime was successfully installed')


def install_python(args):
    info('Installing Python frontend')
    inc_indent()

    need_cmd(['poetry', 'pip3'])

    now = time.time()

    source_dir = os.path.join(ROOT_DIR, 'frontends', 'python')
    build_dir = os.path.join(source_dir, 'dist')

    def _remove_aardwolf_tools_dev_dependency(fh):
        return ''.join(filter(lambda line: not line.startswith('aardwolf_tools'), fh.readlines()))

    # Remove aardwolf_tools development (!) dependency from the pyproject in
    # order to be able to successfully build the package using poetry. This
    # should work out of the box since dev-dependencies do not affect the
    # package distribution. See the following issue for more info:
    # https://github.com/python-poetry/poetry/issues/266
    pyproject_toml = change_file_temporarily(
        os.path.join(source_dir, 'pyproject.toml'),
        _remove_aardwolf_tools_dev_dependency)

    info('Compile')
    run_cmd('poetry', ['build'], cwd=source_dir)

    info('Install')
    install_python_package(args, build_dir, now)

    pyproject_toml.restore()

    dec_indent()
    info('Python frontend was successfully installed')


def install_python_package(args, dist_dir, modified_time):
    for filename in glob.glob(os.path.join(dist_dir, '*.tar.gz')):
        modified = os.path.getmtime(filename)

        if modified > modified_time:
            filename = os.path.basename(filename)
            shutil.copy(os.path.join(dist_dir, filename),
                        os.path.join(args.destination, filename))

            # By default, Python frontend assumes usage in a virtual
            # environment, so it does not install itself system-wide. This
            # can be overrode by --system-python flag.
            if args.system_python:
                run_cmd('pip3', ['install', '--user',
                                 os.path.join(args.destination, filename)])


class TemporaryChange:
    def __init__(self, backup, original):
        self.backup_ = backup
        self.original_ = original

    def restore(self):
        shutil.move(self.backup_, self.original_)


def change_file_temporarily(filename, change):
    backup = filename + '.bak'

    shutil.copy2(filename, backup)

    with open(filename) as fh:
        changed = change(fh)

    with open(filename, 'w') as fh:
        fh.write(changed)

    return TemporaryChange(backup, filename)


if __name__ == "__main__":
    main()
