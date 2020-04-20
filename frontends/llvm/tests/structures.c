struct foo {
    int bar;
    int baz;
};

struct nested {
    struct foo foo;
};

// AARD: function: main
int main()
{
    // AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 14:20-14:20]
    struct foo foo = { .bar = 3, .baz = 42 };
    // AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %3 [@1 16:17-16:17]
    struct foo *foo_ptr = &foo;

    // AARD: #1:3 -> #1:4  ::  defs: %1.%4 / uses: %1.%5 [@1 19:13-19:13]
    foo.bar = foo.baz;
    // AARD: #1:4 -> #1:5  ::  defs: %2.%4 / uses: %2.%5 [@1 21:18-21:18]
    foo_ptr->bar = foo_ptr->baz;

    // AARD: #1:5 -> #1:6  ::  defs: %3 / uses: %1 [@1 24:5-24:5]
    struct nested nested = { .foo = foo };

    // AARD: #1:6 -> #1:7  ::  defs: %3.%4.%5 / uses:  [@1 27:20-27:20]
    nested.foo.baz = 42;

    // AARD: #1:7 ->   ::  defs:  / uses:  [@1 30:5-30:531 { ret }
    return 0;
}

// AARD: @1 = structures.c
// AARD: SKIP
