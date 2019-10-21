void bubble_sort(int *values, unsigned n)
{
    unsigned i;
    int temp;
    unsigned char swapped = 0;

    do {
        for (i = 0; i < n - 1; i++) {
            swapped = 0;

            if (values[i] > values[i + 1]) {
                temp = values[i];
                values[i] = values[i + 1];
                values[i + 1] = temp;
                swapped = 1;
            }
        }

        n = n - 1;
    } while (swapped);
}
