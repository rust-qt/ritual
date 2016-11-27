class {class_name} : public QObject {{
  Q_OBJECT
public:
  {class_name}() : m_func(0), m_data(0) {{ }}
  void set({func_arg}, void* data) {{
    m_func = func;
    m_data = data;
  }}

public slots:
  void custom_slot({method_args}) {{
    m_func({func_args});
  }}

private:
  {func_field};
  void* m_data;
}};
