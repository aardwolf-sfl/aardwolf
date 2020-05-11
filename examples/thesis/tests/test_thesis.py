import aardwolf

from thesis import scale_minmax


def test_scale_positive():
    values = [1, 3, 2]
    expected = [0, 1, 0.5]

    assert scale_minmax(values) == expected

def test_scale_negative():
    values = [-1, -3, -2]
    expected = [1, 0, 0.5]

    assert scale_minmax(values) == expected

def test_scale_mixed():
    values = [-1, 3, 1]
    expected = [0, 1, 0.5]

    assert scale_minmax(values) == expected

def test_scale_equal():
    values = [2, 2, 2]
    expected = [1, 1, 1]

    assert scale_minmax(values) == expected


aardwolf.wrap_module(starts_with='test_')
