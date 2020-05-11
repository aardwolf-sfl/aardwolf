def get_range(values):
    min = max = values[0]

    for x in values:
        if x < min:
            min = x
        if x > max:
            max = x

    return min, max


def safe_div(nom, denom):
    if denom == 0:
        return float('inf')
    else:
        return nom / denom


def scale_minmax(values):
    min, max = get_range(values)

    if min == max:
        min = 0

    scaled = []
    for x in values:
        y = safe_div(x - min, max - min)
        scaled.append(y)

    return scaled
