// Class generated by ritual.
// See the template at "ritual/templates/c_lib/qt_slot_wrapper.h".
class {class_name} : public QObject {{
    Q_OBJECT
public:
    {class_name}(QObject* parent, {callback_arg}, void (*deleter)(void*), void* data)
    : QObject(parent)
    {{
        set(callback, deleter, data);
    }}

    void set({callback_arg}, void (*deleter)(void*), void* data) {{
        m_callback.set(callback, deleter, data);
    }}

public Q_SLOTS:
    void slot_({method_args}) {{
        auto callback = m_callback.get();
        if (callback) {{
            callback({func_args});
        }}
    }}

private:
    ritual::Callback<{callback_type}> m_callback;
}};
