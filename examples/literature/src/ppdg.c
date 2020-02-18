int findmax(int *values, int n)
{
    int i = 0;
    int max = 0;

    while (i < n) {
        int v = values[i];
        if (v > max) {
            max = v;
        }

        i++;
    }

    return max;
}
