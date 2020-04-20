int GLOBAL = 0;

// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs: %1 / uses: %1 [@1 7:11-7:11]
    GLOBAL++;

    // AARD: #1:2 ->   ::  defs:  / uses:  [@1 10:5-10:5]  { ret }
    return 0;
}

// AARD: @1 = globals.c
