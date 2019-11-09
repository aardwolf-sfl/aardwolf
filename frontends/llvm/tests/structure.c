#include <math.h>

struct complex {
    float real;
    float imag;
};

void complex_init(struct complex *self, float real, float imag)
{
    self->real = real;
    self->imag = imag;
}

float complex_abs(struct complex *self)
{
    return sqrtf(self->real * self->real + self->imag * self->imag);
}
