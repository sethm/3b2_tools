#include <stdio.h>
#include <stdint.h>

#define PD_TAG(vaddr)     (((vaddr >> 13) & 0xf) | ((vaddr >> 14) & 0xfff0))
#define PD_IDX(vaddr)     (((vaddr >> 11) & 3) | ((vaddr >> 15) & 4))

void print_paged_vaddr(uint32_t vaddr) {
    printf("     Paged Virtual Address 0x%08x\n\n", vaddr);

    printf(" 31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00\n");
    printf("+-----+-----------------------------------.--+-----------.--.--+--------------------------------+\n");
    printf("|");

    for (int i = 31; i >= 0; i--) {
        if (i == 30 || i == 17 || i == 18 || i == 13 || i == 12 || i == 11) {
            printf(" %d|", (vaddr >> i) & 1);
        } else if (i == 0) {
            printf(" %d", (vaddr >> i) & 1);
        } else {
            printf(" %d ", (vaddr >> i) & 1);
        }
    }

    printf("|\n");
    printf("+-----+-----------------------------------.--+-----------.--.--+--------------------------------+\n");

    printf("\n");

    printf("    TAG=%04x    IDX=%04x\n", PD_TAG(vaddr), PD_IDX(vaddr));
}

int main(int argc, char **argv) {
    uint32_t vaddr;
    int scanret;

    if (argc != 2) {
        fprintf(stderr, "Usage: check_vaddr <vaddr>\n");
        return 1;
    }

    scanret = sscanf(argv[1], "%x", &vaddr);

    if (scanret <= 0 || scanret == EOF) {
        fprintf(stderr, "Unable to parse vaddr.\n");
        return -1;
    }

    print_paged_vaddr(vaddr);

    return 0;
}
