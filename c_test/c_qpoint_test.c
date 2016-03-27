#include "qtcw_QPoint.h"
#include <stdio.h>
#include <malloc.h>

int main() {
  QPoint point;
  QPoint_constructor_xpos_ypos(2, 4, &point);
  printf("test %d %d\n", QPoint_x(&point), QPoint_y(&point));
  QPoint_destructor(&point);
  return 0;
}
