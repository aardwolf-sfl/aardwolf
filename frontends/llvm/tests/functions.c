// AARD: function: square
// AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 3:16-3:16]  { arg }
int square(int n) {
    // AARD: #1:2 ->   ::  defs:  / uses: %1 [@1 5:5-5:5]  { ret }
    return n * n;
}

// AARD: function: twice
// AARD: #1:3 -> #1:4  ::  defs: %2 / uses:  [@1 10:15-10:15]  { arg }
int twice(int n) {
    // AARD: #1:4 ->   ::  defs:  / uses: %2 [@1 12:5-12:5]  { ret }
    return 2 * n;
}

// AARD: function: is_positive
// AARD: #1:5 -> #1:6  ::  defs: %3 / uses:  [@1 17:22-17:22]  { arg }
int is_positive(int *n) {
    // AARD: #1:6 -> #1:7, #1:8  ::  defs:  / uses: %3[] [@1 19:9-19:9]
    if (*n >= 0) {
        // AARD: #1:7 -> #1:10  ::  defs: %4 / uses:  [@1 21:9-21:9]
        return 1;
    } else {
        // AARD: #1:8 -> #1:9  ::  defs: %3[] / uses: %3[] [@1 24:12-24:12]
        *n = -(*n);
        // AARD: #1:9 -> #1:10  ::  defs: %4 / uses:  [@1 26:9-26:9]
        return 0;
    }
    // AARD: #1:10 ->   ::  defs:  / uses: %4 [@1 29:1-29:1]  { ret }
}

// AARD: function: main
int main()
{
    // AARD: #1:11 -> #1:12  ::  defs: %5 / uses:  [@1 35:9-35:9]
    int n = 3;

    // AARD: #1:12 -> #1:13  ::  defs: %6 / uses: %5 [@1 41:16-41:16]  { call }
    // AARD: #1:13 -> #1:14  ::  defs: %7 / uses: %6 [@1 41:9-41:9]  { call }
    // AARD: #1:14 -> #1:15  ::  defs: %8 / uses:  [@1 41:28-41:28]  { call }
    // AARD: #1:15 -> #1:16  ::  defs: %5 / uses: %7, %8 [@1 41:7-41:7]
    n = square(twice(n)) - square(2);

    // AARD: #1:16 -> #1:17  ::  defs: %9 / uses: %5 [@1 44:5-44:5]  { call }
    is_positive(&n);

    // AARD: #1:17 ->   ::  defs:  / uses:  [@1 47:5-47:5]  { ret }
    return 0;
}

// AARD: @1 = functions.c
