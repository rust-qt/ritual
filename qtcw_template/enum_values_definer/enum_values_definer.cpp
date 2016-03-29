#include <QDebug>
#include <QCoreApplication>
#include <QFile>

//include everything
#include <QtCore>

#define ADD(define_name, value) file.write(QString("#define QTCW_EV_%1 %2\n").arg(define_name).arg(value).toLatin1());

int main(int argc, char *argv[]) {
  QCoreApplication app(argc, argv);
  QStringList args = app.arguments();
  if (args.count() < 2) {
    qFatal("enum_values_definer: no filename supplied.");
    return 1;
  }
  qDebug() << "enum_values_definer: Generating file: " << args[1];
  QFile file(args[1]);
  if (!file.open(QFile::WriteOnly)) {
    qFatal("enum_values_definer: can't open file.");
    return 2;
  }

  #include "values_list.h"

  return 0;
}
