#ifndef OVERLOADED_H
#define OVERLOADED_H

#include "moqt_core_exports.h"
#include "QString.h"
#include "QPoint.h"

class MOQT_CORE_EXPORT Overloaded {
public:
    // functions differ only by argument type
    Overloaded(int x);
    Overloaded(float x);
    Overloaded(QString x);

    // functions differ only by const-ness
    int at(int index);
    int at(int index) const;

    void setPos(int x, int y);
    void setPos(QPoint pos, bool flag);

    void match();
    void match(int x);
};

#endif //OVERLOADED_H
