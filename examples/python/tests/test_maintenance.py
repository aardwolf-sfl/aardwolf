from datetime import datetime
import maintenance
from maintenance import Entity, process

import aardwolf
aardwolf.install(maintenance, '.aardwolf')


def make_standardized(standard, name):
    if standard == 'A':
        timestamp = datetime.now().strftime('%y%m%d')
        return f'A_{name}_{timestamp}'
    elif standard == 'B':
        timestamp = datetime.now().strftime('%Y%m%d')
        return f'B{name}{timestamp}'
    else:
        raise Exception('Unknown standard')


def is_deep_equal(actual, expected, message):
    result = len(actual) == len(expected)
    for actual_task, expected_task in zip(actual, expected):
        result = result and (actual_task == expected_task)

    return result


def test_tasks_threshold():
    e1 = Entity(1, 'A', 3, 2, 10)
    e2 = Entity(2, 'A', 3, 2, 10)

    e1.add_task('e1t1', 2)
    e1.add_task('e1t2', 1)
    e2.add_task('e2t1', 2)
    e2.add_task('e2t2', 4)

    expected = [(1, make_standardized('A', 'e1t1')),
                (1, make_standardized('A', 'e1t2'))]
    assert is_deep_equal(process([e1, e2]), expected, 'T1')


def test_standard_names():
    e1 = Entity(1, 'A', 3, 1, 10)
    e2 = Entity(2, 'B', 3, 1, 10)

    e1.add_task('e1t1', 2)
    e2.add_task('e2t1', 2)

    expected = [(1, make_standardized('A', 'e1t1')),
                (2, make_standardized('B', 'e2t1'))]
    assert is_deep_equal(process([e1, e2]), expected, 'T2')


def test_waiting_threshold():
    e1 = Entity(1, 'A', 3, 5, 3)
    e2 = Entity(2, 'A', 3, 5, 4)

    e1.add_task('e1t1', 10)
    e2.add_task('e2t1', 10)

    expected = [(1, make_standardized('A', 'e1t1'))]
    assert is_deep_equal(process([e1, e2]), [], 'T3')
    assert is_deep_equal(process([e1, e2]), [], 'T3')
    assert is_deep_equal(process([e1, e2]), [], 'T3')
    assert is_deep_equal(process([e1, e2]), expected, 'T3')


def test_critical_tasks():
    e1 = Entity(1, 'A', 3, 2, 10)
    e2 = Entity(2, 'A', 3, 2, 10)

    e1.add_task('e1t1', 1)
    e2.add_task('e2t1', 2)

    expected = [(1, make_standardized('A', 'e1t1'))]
    assert is_deep_equal(process([e1, e2]), [], 'T4')
    assert is_deep_equal(process([e1, e2]), expected, 'T4')


aardwolf.wrap_module(starts_with='test_')
