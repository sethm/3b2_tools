#include <stdio.h>
#include <stdint.h>

#define P(sd)             ((sd) & 1)
#define M(sd)             (((sd) >> 1) & 1)
#define C(sd)             (((sd) >> 2) & 1)
#define CC(sd)            (((sd) >> 3) & 1)
#define T(sd)             (((sd) >> 4) & 1)
#define R(sd)             (((sd) >> 5) & 1)
#define V(sd)             (((sd) >> 6) & 1)
#define I(sd)             (((sd) >> 7) & 1)
#define MAX_OFF(sd)       ((((sd) >> 10) & 0x1fff) + 1)
#define ACC(sd)           (((sd) >> 24) & 0xff)

int main(int argc, char **argv) {
    uint32_t sd;
    int scanret;

    if (argc != 2) {
        fprintf(stderr, "Usage: check_sd <descriptor>\n");
        return 1;
    }

    scanret = sscanf(argv[1], "%x", &sd);

    if (scanret <= 0 || scanret == EOF) {
        fprintf(stderr, "Unable to parse segment descriptor.\n");
        return -1;
    }

    printf("     Segment Descriptor 0x%08x\n\n", sd);
    printf("Present:     %d\n", P(sd));
    printf("Modified:    %d\n", P(sd));
    printf("Contiguous:  %d\n", C(sd));
    printf("Cacheable:   %d\n", CC(sd));
    printf("Object Trap: %d\n", T(sd));
    printf("Referenced:  %d\n", R(sd));
    printf("Valid:       %d\n", V(sd));
    printf("Indirect:    %d\n", I(sd));
    printf("Max Offset:  %04x\n", MAX_OFF(sd));
    printf("Access:      %02x\n", ACC(sd));

    return 0;
}
