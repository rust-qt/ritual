#include "moqt_core_utils.h"

int ctrt1_abs(int x) {
  if (x >= 0) {
    return x;
  } else {
    return -x;
  }
}

const char* ctrt1_version() {
  return "0.0.1";
}
