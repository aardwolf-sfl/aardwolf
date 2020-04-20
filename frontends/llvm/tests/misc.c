int array[3];

struct foo {
    int bar;
};

struct foo mixed[3];

// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs: %1[] / uses:  [@1 13:14-13:14]
    array[0] = 100;
    // AARD: #1:2 -> #1:3  ::  defs: %1[] / uses:  [@1 15:14-15:14]
    array[1] = 200;
    // AARD: #1:3 -> #1:4  ::  defs: %1[] / uses:  [@1 17:14-17:14]
    array[2] = 300;

    // AARD: #1:4 -> #1:5  ::  defs: %2 / uses:  [@1 20:9-20:9]
    int i = 0;
    // AARD: #1:5 -> #1:6  ::  defs: %3 / uses:  [@1 22:9-22:9]
    int j = 1;

    // AARD: #1:6 -> #1:7  ::  defs: %4[%2, %3].%5 / uses: %2, %3 [@1 25:22-25:22]
    mixed[i + j].bar = i + j;

    // AARD: #1:7 ->   ::  defs:  / uses:  [@1 28:5-28:5]  { ret }
    return 0;
}

// AARD: @1 = misc.c
