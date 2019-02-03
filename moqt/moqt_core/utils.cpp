#include "utils.h"

int moqt_abs(int x) {
    if (x >= 0) {
        return x;
    } else {
        return -x;
    }
}

const char *moqt_core_version() {
    return "0.0.1";
}
