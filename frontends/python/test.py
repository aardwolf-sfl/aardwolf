#!/bin/python3

import runpy
import os

TEST_RUNNER = os.path.join(os.path.dirname(
    os.path.realpath(__file__)), 'tests', '__init__.py')
runpy.run_path(TEST_RUNNER)
