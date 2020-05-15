import aardwolf
import pytest


def pytest_runtest_call(item):
    if aardwolf.use_aardwolf():
        aardwolf.write_external(item.name)


@pytest.hookimpl(tryfirst=True, hookwrapper=True)
def pytest_runtest_makereport(item, call):
    if aardwolf.use_aardwolf():
        outcome = yield
        result = outcome.get_result()

        if result.when == 'call':
            aardwolf.write_test_status(item.name, result.outcome == 'passed')
