#include "qtcw_QRect.h"
#include <stdio.h>
#include <malloc.h>
#include <assert.h>

int main() {
  QRect rect1;
  QRect_constructor_x_y_width_height(1, 2, 3, 4, &rect1);
  assert(QRect_height(&rect1) == 4);
  QRect_destructor(&rect1);

  QRect* rect2 = malloc(sizeof(QRect));

  QRect_constructor_x_y_width_height(5, 6, 7, 8, rect2);
  assert(QRect_height(rect2) == 8);
  QRect_destructor(rect2);

  free(rect2);

  return 0;
}
