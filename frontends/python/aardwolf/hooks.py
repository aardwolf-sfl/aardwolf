import sys
import os
import types

from importlib import invalidate_caches, reload
from importlib.abc import Loader, MetaPathFinder
from importlib.util import spec_from_file_location, find_spec
from importlib.machinery import SourceFileLoader

from .pipeline import process_file


class AardwolfMetaFinder(MetaPathFinder):
    def __init__(self, package, outdir=None):
        self.package_ = package
        self.outdir_ = outdir

    def find_spec(self, fullname, path, target=None):
        split = fullname.split('.')

        # If the requested module comes from the user package to be analysed and
        # instrumented, then use Aardwolf loader. Otherwise, use the default
        # one.
        if split[0] == self.package_:
            # If the the import is top-level, `path` is None and we will use
            # current working directory.
            if path is None or path == '':
                path = [os.getcwd()]

            # Use the last part of the import.
            name = split[-1]

            for entry in path:
                if os.path.isdir(os.path.join(entry, name)):
                    # Directory module with child modules.
                    filename = os.path.join(entry, name, "__init__.py")
                    submodule_locations = [os.path.join(entry, name)]
                else:
                    # Assume file module.
                    filename = os.path.join(entry, name + '.py')
                    submodule_locations = None

                # Check if the source file actually exists.
                if not os.path.isfile(filename):
                    continue

                return spec_from_file_location(
                    fullname,
                    filename,
                    loader=AardwolfLoader(filename, self.outdir_),
                    submodule_search_locations=submodule_locations)

            # Unable to find the source file. Use the default machinery.
            return None
        else:
            # Use default machinery.
            return None


class AardwolfLoader(Loader):
    def __init__(self, filename, outdir=None):
        self.filename_ = filename
        if outdir is None:
            self.outdir_ = os.environ.get('AARDWOLF_DATA_DEST', os.getcwd())
        else:
            self.outdir_ = outdir

    def create_module(self, spec):
        return None  # use default module creation semantics

    def exec_module(self, module):
        code = process_file(self.filename_, self.outdir_)
        exec(code, vars(module))


class PackageError(Exception):
    def __init__(self, message):
        self.message = message

    @staticmethod
    def not_found(name):
        return PackageError(f'Package "{name}" was not found by Aardwolf.')

    @staticmethod
    def no_source(name):
        return PackageError(f'Package "{name}" does not have its source code available, which is required for Aardwolf.')


def install(package, outdir=None):
    if isinstance(package, str):
        # Try to get the specification for the package.
        spec = find_spec(package)
        reload_package = False
    elif isinstance(package, types.ModuleType):
        # IMPORTANT: Reloading does not reload previously imported objects using
        # from <module> import <object>
        spec = find_spec(package.__name__)
        reload_package = True

    if spec is None:
        raise PackageError.not_found(package)

    # Only accept packages with source file (that is, not packages having only
    # bytecode).
    if not issubclass(type(spec.loader), SourceFileLoader):
        raise PackageError.no_source(package)

    # Insert Aardwolf finder to the beginning of meta_path.
    sys.meta_path.insert(0, AardwolfMetaFinder(spec.name, outdir))

    if reload_package:
        reload(package)
