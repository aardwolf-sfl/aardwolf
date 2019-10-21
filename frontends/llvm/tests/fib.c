int fib(int n)
{
    int a, b, i, t;
    a = 0;
    b = 1;

    for (i = 1; i < n; i++) {
        t = a + b;
        a = b;
        b = t;
    }

    return b;
}
