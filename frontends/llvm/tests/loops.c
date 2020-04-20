// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:10-5:10]
    char condition = 1;
    // AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 7:9-7:9]
    int n = 3;

    // AARD: #1:3 -> #1:4, #1:5  ::  defs:  / uses: %1 [@1 10:5-10:5]
    while (condition) {
        // AARD: #1:4 -> #1:3  ::  defs: %2 / uses: %2 [@1 12:10-12:10]
        n++;
    }

    // AARD: #1:5 -> #1:6  ::  defs: %3 / uses:  [@1 17:14-17:14]
    // AARD: #1:6 -> #1:7, #1:9  ::  defs:  / uses: %2, %3 [@1 17:5-17:5]
    for (int i = 0; i < n; i++) {
        // AARD: #1:7 -> #1:8  ::  defs: %1 / uses: %3 [@1 19:19-19:19]
        condition = i;
        // AARD: #1:8 -> #1:6  ::  defs: %3 / uses: %3 [@1 17:29-17:29]
    }

    do {
        // AARD: #1:9 -> #1:10  ::  defs: %2 / uses: %2 [@1 25:10-25:10]
        n++;
        // AARD: #1:10 -> #1:11, #1:9  ::  defs:  / uses: %1 [@1 27:5-27:5]
    } while (condition);

    // AARD: #1:11 ->   ::  defs:  / uses:  [@1 30:5-30:5]  { ret }
    return 0;
}

// AARD: @1 = loops.c
