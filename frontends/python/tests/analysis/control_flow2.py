
# AARD: function: foo
# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:9-4:15]  { arg }
def foo(device):
    # AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 6:5-6:16]
    counter = 0

    # AARD: #1:3 -> #1:4, #1:5  ::  defs:  / uses: %1.%3, %1.%4 [@1 9:8-9:51]
    if device.waiting_ >= device.waiting_threshold:
        # AARD: #1:4 ->   ::  defs:  / uses:  [@1 11:9-11:20]  { ret }
        return True

    # AARD: #1:5 -> #1:6, #1:7  ::  defs: %5 / uses: %1.%6 [@1 14:9-14:13]
    for task in device.tasks_:
        # AARD: #1:6 -> #1:5, #1:8  ::  defs:  / uses: %1.%8, %5.%7 [@1 16:12-16:54]
        if task.priority_ < device.priority_threshold:
            # AARD: #1:8 -> #1:5  ::  defs: %2 / uses: %2 [@1 18:13-18:25]
            counter += 1

    # AARD: #1:7 ->   ::  defs:  / uses: %1.%9, %2 [@1 21:5-21:45]  { ret }
    return counter >= device.tasks_threshold

# AARD: @1 = control_flow2.py
