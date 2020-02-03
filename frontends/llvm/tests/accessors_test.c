struct foo {
    int n;
};

struct foo global[5];

struct baz {
    struct foo foo;
};

int bar(int i) {
    return global[i].n;
}

int quo() {
    struct baz b;
    return b.foo.n;
}
