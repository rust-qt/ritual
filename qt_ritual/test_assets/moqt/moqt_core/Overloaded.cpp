#include "Overloaded.h"

Overloaded::Overloaded(int x) {}
Overloaded::Overloaded(float x) {}
Overloaded::Overloaded(QString x) {}

// functions differ only by const-ness
int Overloaded::at(int index) {
    return 0;
}
int Overloaded::at(int index) const {
    return 0;
}
void Overloaded::setPos(int x, int y) {}
void Overloaded::setPos(QPoint pos, bool flag) {}

void Overloaded::match() {}
void Overloaded::match(int x) {}
