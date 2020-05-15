import os
from collections import OrderedDict


def unique(values):
    return list(OrderedDict.fromkeys(values))


def use_aardwolf():
    # Check if inside Aardwolf "environment". If data destination directory is
    # set, we suppose that Aardwolf data should be generated, i.e., that
    # Aardwolf should be used.
    return 'AARDWOLF_DATA_DEST' in os.environ


class IdMap:
    def __init__(self):
        self.data_ = dict()

    def get(self, value):
        if value not in self.data_:
            self.data_[value] = len(self.data_) + 1

        return self.data_[value]

    def get_checked(self, value):
        orig_len = len(self.data_)
        index = self.get(value)
        return index, len(self.data_) != orig_len


class Counter:
    def __init__(self):
        self.data_ = dict()

    def get_inc(self, value):
        if value not in self.data_:
            self.data_[value] = 0

        self.data_[value] += 1
        return self.data_[value]
