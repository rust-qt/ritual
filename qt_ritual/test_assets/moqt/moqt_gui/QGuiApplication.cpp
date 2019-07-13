#include "QGuiApplication.h"

QGuiApplication::QGuiApplication(int &argc, char **argv, int flags) :
    QCoreApplication(argc, argv, flags) {

}

int QGuiApplication::exec() {
    return 0;
}
