This is work in progress, so the API will significantly change in the future.
Some methods are missing, and some are inconvenient to use.
Some methods are unsafe even though they are not marked as unsafe.
Users must carefully track ownership of the objects, as usual Rust guarantees
do not take effect. This will hopefully improve in the future.
Please report any issues to the
[issue tracker](https://github.com/rust-qt/ritual/issues).

# Starting up

Qt requires an application object to be constructed at the beginning of the application.
(Some classes may be used without it,
The application object needs `argc` and `argv` available in `main` function in C++.
It's a bit tricky to do it in Rust, where `argc` and `argv` are not available.
`CoreApplication::init` is a convenience function that performs proper
creation of the application object and terminates the process with the appropriate return code
when the application exists:

```rust,no_run
extern crate qt_core;
use qt_core::core_application::CoreApplication;

fn main() {
  CoreApplication::init(|app| {
    // initialization goes here
    CoreApplication::exec()
  })
}
```

Note that if you use `qt_gui` or `qt_widgets` crates, you should use
`qt_gui::gui_application::GuiApplication` and `qt_widgets::application::Application`
respectively instead of `CoreApplication`.

`CoreApplication::exec` starts the main event loop. After your initialization code finishes,
any other Rust code will only be executed by Qt if you bind it to a slot:

```rust
extern crate qt_core;
use qt_core::core_application::CoreApplication;
use qt_core::variant::Variant;
use qt_core::variant_animation::VariantAnimation;
use qt_core::connection::Signal;
use qt_core::slots::SlotVariantRef;

fn main() {
  CoreApplication::init(|app| {
    let slot1 = SlotVariantRef::new(|value| {
      println!("value_changed: {}",
               value.to_string().to_std_string());
    });

    let mut animation = VariantAnimation::new();
    animation.signals().value_changed().connect(&slot1);
    animation
        .signals()
        .finished()
        .connect(&app.slots().quit());
    animation.set_start_value(&Variant::new0(1));
    animation.set_end_value(&Variant::new0(5));
    animation.set_duration(5000);
    animation.start(());
    CoreApplication::exec()
  })
}
```

# Naming

Names of Qt's classes and methods are modified according to Rust's naming conventions.
`Q` prefix is removed.
Each of Qt's include files is converted to a submodule. Original C++ names are always
listed in the documentation, so you may search for the Rust equivalents by original names.

# Types and ownership

Qt crates use two ways of handling ownership of C++ objects.

Value-like types (`QString`, `QVector`, etc.) are represented by owned struct types
(e.g. `qt_core::string::String`) in Rust. The value is stored in the memory Rust itself
reserves for the struct. `Drop` implementation of the type will call C++ destructor to
ensure proper de-initialization of the value. It's not possible to transfer ownership
of such object to C++ side.

All other types are stored in C++ heap and handled using raw and smart pointers.
Raw pointer types (e.g. `*mut qt_core::object::Object`) are the same pointers as in C++.
There is no guarantee that the pointer is valid at any time, and the null pointer
indicates lack of an object.  There is also no information about ownership in raw pointers.
Some C++ functions may return a raw object and expect the caller to take ownership, while
other functions keep ownership and may delete the object at any time. As in C++, the caller
needs to refer to the function's documentation and handle ownership manually.

When it's determined that the ownership of the object belongs to the caller
(e.g. in a constructor), the raw pointer `*mut T` is wrapped into `cpp_utils::CppBox<T>`.
This struct owns the object and will delete it when dropped. It allows to move the raw pointer
out in case you need to transfer the ownership back to C++ side.

References (`&T` and `&mut T`) in Qt crates are not very different from raw pointers.
They appear in the same places references were used in C++, but they can't hold any guarantees
Rust usually enforces for references. Lifetimes of references are set trivially: all input
references must be valid for the same lifetime, and output references have the same lifetime
as input references. If there are no input references, output references have `'static`
lifetime.

It should be expected that raw pointers will be replaced with `CppBox`es and references,
and references will hold their guarantees. However, this requires manual annotation of methods,
so it's not easy to make this improvement.
