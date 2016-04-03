//include everything
#include <QtCore>
#include <QJsonObject>

class Extractor {
public:
  Extractor() {
  }

  template<typename T>
  void add_class(const char* name) {
    m_class_sizes[QString(name)] = sizeof(T);
  }

  template<typename T>
  void add_enum_value(const char* enum_name, const char* value_name, T value) {
    m_enum_values[QString(enum_name)][QString(value_name)] = static_cast<int>(value);
  }

  void save(const QString& output_file_name) {
    QFile file(output_file_name);
    if (!file.open(QFile::WriteOnly)) {
      qFatal("cpp_info_extractor: can't open file.");
      qApp->exit(2);
    }
    QJsonObject enum_values;
    foreach(QString key, m_enum_values.keys()) {
      QJsonObject v;
      foreach(QString key2, m_enum_values[key].keys()) {
        v[key2] = m_enum_values[key][key2];
      }
      enum_values[key] = v;
    }

    QJsonObject class_sizes;
    foreach(QString key, m_class_sizes.keys()) {
      class_sizes[key] = m_class_sizes[key];
    }

    QJsonObject root;
    root["enum_values"] = enum_values;
    root["class_sizes"] = class_sizes;
    file.write(QJsonDocument(root).toJson());
  }

private:
  QString m_output_file_name;
  QHash< QString, QHash<QString, int> > m_enum_values;
  QHash< QString, int > m_class_sizes;

};






void add(QFile* file, QString name, int value) {
  file->write(QString("#define QTCW_sizeof_%1 %2\n").arg(name).arg(value).toLatin1());
}

#define ADD(define_name, class_name) add(&file, #define_name, sizeof(class_name))

int main(int argc, char *argv[]) {
  QCoreApplication app(argc, argv);
  QStringList args = app.arguments();
  if (args.count() < 2) {
    qFatal("cpp_info_extractor: no filename supplied.");
    return 1;
  }
  qDebug() << "cpp_info_extractor: Generating file: " << args[1];
  Extractor e;

  #include "cpp_info_extractor_actions.h"

  e.save(args[1]);
  return 0;
}
