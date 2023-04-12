#ifndef RITUAL_GLOBAL_H
#define RITUAL_GLOBAL_H

// This header includes system headers and declares functions
// required by all regular headers of the library.

// for fixed size integer types
#include <stdint.h>

// for default cpp_checker tests
#include <iostream>
#include <assert.h>

// placement new statements require this
#include <new>

// for exit()
#include <cstdlib>

#ifdef _WIN32
    #define RITUAL_EXPORT __declspec(dllexport)
#else
    #define RITUAL_EXPORT
#endif

#define ritual_assert(x) \
    if (!(x)) { \
        std::cout << "assertion failed: " << #x << "\n"; \
        exit(1); \
    }

namespace ritual {
    // Calls destructor of `T` class. This template function
    // is necessary because it's not possible to use `x->~T()`
    // syntax directly if `T` contains `::`.
    template<typename T>
    void call_destructor(T* x) {
        x->~T();
    }

    template<class T>
    class Callback {
    public:
        Callback() {
            m_data = nullptr;
            m_deleter = nullptr;
            m_callback = nullptr;
        }
        ~Callback() {
            if (m_deleter) {
                m_deleter(m_data);
            }
        }
        void set(T callback, void (*deleter)(void*), void* data) {
            if (m_deleter) {
                m_deleter(m_data);
            }
            m_callback = callback;
            m_deleter = deleter;
            m_data = data;
        }
        T get() const { return m_callback; }
        void* data() const { return m_data; }

    private:
        void* m_data;
        void (*m_deleter)(void*);
        T m_callback;
    };
}

#endif // RITUAL_GLOBAL_H
