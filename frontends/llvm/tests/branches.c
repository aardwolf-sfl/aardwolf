// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:10-5:10]
    char condition = 1;
    // AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 7:9-7:9]
    int value = 3;

    // AARD: #1:3 -> #1:4, #1:5  ::  defs:  / uses: %1 [@1 10:9-10:9]
    if (condition) {
        // AARD: #1:4 -> #1:5  ::  defs: %2 / uses:  [@1 12:15-12:15]
        value = 1;
    }

    // AARD: #1:5 -> #1:6, #1:7  ::  defs:  / uses: %1 [@1 16:9-16:9]
    if (condition) {
        // AARD: #1:6 -> #1:8  ::  defs: %2 / uses:  [@1 18:15-18:15]
        value = 1;
    } else {
        // AARD: #1:7 -> #1:8  ::  defs: %2 / uses:  [@1 21:15-21:15]
        value = 2;
    }

    // AARD: #1:8 -> #1:10, #1:9  ::  defs:  / uses: %1 [@1 25:9-25:9]
    if (condition) {
        // AARD: #1:9 -> #1:13  ::  defs: %2 / uses:  [@1 27:15-27:15]
        value = 1;
    // AARD: #1:10 -> #1:11, #1:12  ::  defs:  / uses: %2 [@1 29:16-29:16]
    } else if (value > 3) {
        // AARD: #1:11 -> #1:13  ::  defs: %2 / uses:  [@1 31:15-31:15]
        value = 3;
    } else {
        // AARD: #1:12 -> #1:13  ::  defs: %2 / uses:  [@1 34:15-34:15]
        value = 2;
    }

    // AARD: #1:13 -> #1:14, #1:15, #1:16, #1:17, #1:18  ::  defs:  / uses: %2 [@1 38:5-38:5]
    switch (value) {
        case 1:
            // AARD: #1:14 -> #1:19  ::  defs: %1 / uses:  [@1 41:23-41:23]
            condition = 1;
            break;

        case 2:
            // AARD: #1:15 -> #1:19  ::  defs: %1 / uses:  [@1 46:23-46:23]
            condition = 1;
            break;

        case 3:
            // AARD: #1:16 -> #1:19  ::  defs: %1 / uses:  [@1 51:23-51:23]
            condition = 1;
            break;

        case 4:
            // AARD: #1:17 -> #1:19  ::  defs: %1 / uses:  [@1 56:23-56:23]
            condition = 1;
            break;

        default:
            // AARD: #1:18 -> #1:19  ::  defs: %1 / uses:  [@1 61:23-61:23]
            condition = 0;
            break;
    }

    // AARD: #1:19 -> #1:20, #1:21, #1:22  ::  defs:  / uses: %1 [@1 66:5-66:5]
    switch (condition) {
        case 1:
            // AARD: #1:20 -> #1:21  ::  defs: %1 / uses:  [@1 69:23-69:23]
            condition = 1;

        case 0:
            // AARD: #1:21 -> #1:22  ::  defs: %1 / uses:  [@1 73:23-73:23]
            condition = 1;
            break;
    }

    // AARD: #1:22 ->   ::  defs:  / uses:  [@1 78:5-78:5]  { ret }
    return 0;
}

// AARD: @1 = branches.c
