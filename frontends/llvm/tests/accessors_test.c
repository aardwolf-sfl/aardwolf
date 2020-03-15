struct foo {
    int n;
};

struct foo global[5];

struct baz
{
    struct foo foo;
};

int lorem[3];

int bar(int i)
{
    return global[i].n;
}

int quo()
{
    struct baz b;
    return b.foo.n;
}

void ipsum()
{
    lorem[0] = 100;
    lorem[1] = 200;
    lorem[2] = 300;
}
