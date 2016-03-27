#include "../c_qrect.h"
#include <stdio.h>
#include <malloc.h>

int main() {
  C_QRect rect1;
  c_qrect_construct(&rect1, 1, 2, 3, 4);
  printf("test1 %d\n", c_qrect_height(&rect1));
  c_qrect_destruct(&rect1);

  C_QRect* rect2 = malloc(sizeof(C_QRect));

  c_qrect_construct(rect2, 5, 6, 7, 8);
  printf("test2 %d\n", c_qrect_height(rect2));
  c_qrect_destruct(rect2);

  free(rect2);

  return 0;
}
