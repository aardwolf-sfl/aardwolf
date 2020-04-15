from collections import OrderedDict

def unique(values):
    return list(OrderedDict.fromkeys(values))
