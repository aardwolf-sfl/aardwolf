int range(int *array, int k)
{
    if (k == 0) {
        return 0;
    } else {
        int min = array[0];
        int max = array[0];

        int i;
        for (i = 1; i < k; i++) {
            if (array[i] < min) {
                min = array[i];
            } else if (array[i] > max) {
                max = array[i];
            }
        }

        return max - min;
    }
}
