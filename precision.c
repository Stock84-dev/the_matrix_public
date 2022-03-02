#include <stdio.h>
#include <float.h>

const double min = 240;
const double max = 60000;

//const double min = 0;
//const double max = 1;

#define normalize(x) (((x) - min) / (max - min))
#define denormalize(x) ((x) * (max - min) + min)

const unsigned int scale = 31;
#define to_fixed(x) ((x) * (double)(1<<scale))
#define from_fixed(x) ((double) (x)/(double)(1<<scale))

typedef union {
  float f;
  struct {
    unsigned int mantisa : 23;
    unsigned int exponent : 8;
    unsigned int sign : 1;
  } parts;
} float_cast;

typedef struct {
	unsigned int mantisa : 32;
	unsigned int exponent : 31;
	unsigned int sign : 1;
} parts;


typedef union {
} Price;

unsigned int get_mantissa(double x) {
	unsigned int* ptr = (unsigned int*)&x;
	unsigned int mat = *ptr;
	printf("%b\n", mat);
	printBits(4, &mat);
	return mat;
}

void printBits(size_t const size, void const * const ptr)
{
    unsigned char *b = (unsigned char*) ptr;
    unsigned char byte;
    int i, j;
    
    for (i = size-1; i >= 0; i--) {
        for (j = 7; j >= 0; j--) {
            byte = (b[i] >> j) & 1;
            printf("%u", byte);
        }
    }
    puts("");
}

void main() {
	//float a = 1.4012984643E-45 * 2;
	//float_cast d1 = { .f = a };
	//printf("%d\n", d1.parts.mantisa);

	float m = 242;
	double close = 12345;
	double M = 12346;
	m = normalize(m);
	printf("%.32lf\n", m);
	float_cast d1 = { .f = m };
	printf("%llu\n", d1.parts.mantisa);
	//return;

	//close = normalize(close);
	//M = normalize(M);
	//printf("%x\n", d1.parts.mantisa);
	//unsigned int fm = get_mantissa(m);
	//printf("%lf\n", from_fixed(fm));
	//fm *= 2;




	//printf("%.32lf\n", denormalize(price + one));
	//printf("%.32lf\n", price);
	//unsigned short fixed = to_fixed(price);
	//printf("%u\n", fixed);
	//price = from_fixed(fixed);
	////int n_one = normalize(price);
	////int result = normalized + n_one;

	//price = denormalize(price);
	//printf("%.32lf\n", price);
}
