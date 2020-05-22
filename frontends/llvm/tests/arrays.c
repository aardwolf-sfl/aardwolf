// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs:  / uses: %1 [@1 5:9-5:9]  { call }
    int array[3] = {0, 0, 0}; // compiles to memset(array, 0)

    // AARD: #1:2 -> #1:3  ::  defs: %1[] / uses:  [@1 8:14-8:14]
    array[0] = 100;
    // AARD: #1:3 -> #1:4  ::  defs: %1[] / uses:  [@1 10:14-10:14]
    array[1] = 200;
    // AARD: #1:4 -> #1:5  ::  defs: %1[] / uses:  [@1 12:14-12:14]
    array[2] = 300;

    // AARD: #1:5 -> #1:6  ::  defs: %2 / uses:  [@1 15:9-15:9]
    int i = 0;
    // AARD: #1:6 -> #1:7  ::  defs: %3 / uses:  [@1 17:9-17:9]
    int j = 1;

    // AARD: #1:7 -> #1:8  ::  defs: %1[%2] / uses: %1[%2] [@1 20:14-20:14]
    array[i] = array[i + 1];
    // AARD: #1:8 -> #1:9  ::  defs: %1[%3] / uses: %1[%2, %3] [@1 22:14-22:14]
    array[j] = array[i + j];

    // AARD: #1:9 -> #1:10  ::  defs: %4[] / uses: %2 [@1 26:20-26:20]
    // AARD: #1:10 -> #1:11  ::  defs: %4[] / uses: %3 [@1 26:20-26:20]
    int tuple[2] = {i, j};

    // AARD: #1:11 ->   ::  defs:  / uses:  [@1 29:5-29:5]  { ret }
    return 0;
}

// AARD: @1 = arrays.c
// AARD: SKIP
