#include "moqt_core_exports.h"

namespace ns1 {
    MOQT_CORE_EXPORT int x();

    namespace ns2 {
        MOQT_CORE_EXPORT int x();
        MOQT_CORE_EXPORT int y();

        enum Enum1 {
            Val1,
            Val2,
            Val3
        };
    }

    namespace ns3 {
        MOQT_CORE_EXPORT int a();
        MOQT_CORE_EXPORT int b();

        enum Enum2 {
            Val11 = 1,
            Val12 = 2,
            Val13 = 3,
        };

        namespace ns4 {
            class MOQT_CORE_EXPORT Class1 {
            public:
                Class1(int x) {}
            };
        }
    }

    template<class T>
    class MOQT_CORE_EXPORT Templated1 {
    public:
        T x() { return 0; }
    };

    class MOQT_CORE_EXPORT ClassNs {
    public:
        class MOQT_CORE_EXPORT Class1 {};

        template<class T>
        class MOQT_CORE_EXPORT Templated2 {
        public:
            T y() { return 0; }
        };
    };
}

namespace ignored_ns {
    class MOQT_CORE_EXPORT Class3 {};

    template<class T>
    class MOQT_CORE_EXPORT Templated3 {
    public:
        T get() { return 0; }
    };
};

MOQT_CORE_EXPORT ns1::Templated1<int> func1();
MOQT_CORE_EXPORT ns1::ClassNs::Templated2<bool> func2();
MOQT_CORE_EXPORT ignored_ns::Templated3<int> func3();
