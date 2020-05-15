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

scale_minmax([1, 3, 2])

# scale_minmax([1, 2, 3])
# AARD: statement: #1:1

# AARD: statement: #1:17
# AARD: unsupported data type

# get_range(values)
# AARD: statement: #1:18

# AARD: statement: #1:2
# AARD: unsupported data type

# AARD: statement: #1:3
# AARD: i64: 1
# AARD: i64: 1

# AARD: statement: #1:4
# AARD: i64: 1

# AARD: statement: #1:5

# AARD: statement: #1:8

# AARD: statement: #1:4
# AARD: i64: 3

# AARD: statement: #1:5

# AARD: statement: #1:8

# AARD: statement: #1:9
# AARD: i64: 3

# AARD: statement: #1:4
# AARD: i64: 2

# AARD: statement: #1:5

# AARD: statement: #1:8

# AARD: statement: #1:6

# result of get_range(values)
# AARD: unsupported data type

# min, max = get_range(values)
# AARD: statement: #1:19
# AARD: i64: 1
# AARD: i64: 3

# AARD: statement: #1:20

# scaled = []
# AARD: statement: #1:22
# AARD: unsupported data type

# AARD: statement: #1:23
# AARD: i64: 1

# safe_div(x - min, max - min)
# AARD: statement: #1:24

# def safe_div(nom, denom):
# AARD: statement: #1:10
# AARD: i64: 0

# AARD: statement: #1:11
# AARD: i64: 2

# AARD: statement: #1:12

# AARD: statement: #1:14

# result of safe_div(x - min, max - min)
# AARD: f64: 0.0

# y = safe_div(x - min, max - min)
# AARD: statement: #1:26
# AARD: f64: 0.0

# scaled.append(y)
# AARD: statement: #1:27
# AARD: unsupported data type

# AARD: statement: #1:23
# AARD: i64: 3

# AARD: statement: #1:24

# AARD: statement: #1:10
# AARD: i64: 2

# AARD: statement: #1:11
# AARD: i64: 2

# AARD: statement: #1:12

# AARD: statement: #1:14

# result of safe_div(x - min, max - min)
# AARD: f64: 1.0

# y = safe_div(x - min, max - min)
# AARD: statement: #1:26
# AARD: f64: 1.0

# AARD: statement: #1:27
# AARD: unsupported data type

# AARD: statement: #1:23
# AARD: i64: 2

# AARD: statement: #1:24

# AARD: statement: #1:10
# AARD: i64: 1

# AARD: statement: #1:11
# AARD: i64: 2

# AARD: statement: #1:12

# AARD: statement: #1:14

# result of safe_div(x - min, max - min)
# AARD: f64: 0.5

# y = safe_div(x - min, max - min)
# AARD: statement: #1:26
# AARD: f64: 0.5

# AARD: statement: #1:27
# AARD: unsupported data type

# AARD: statement: #1:25
# AARD: unsupported data type
