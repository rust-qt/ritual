#include "moqt_core_exports.h"

namespace ns1 {
    MOQT_CORE_EXPORT int x() {}

    namespace ns2 {
        MOQT_CORE_EXPORT int x() {}
        MOQT_CORE_EXPORT int y() {}

        enum Enum1 {
            Val1,
            Val2,
            Val3
        };
    };

    namespace ns3 {
        MOQT_CORE_EXPORT int a() {}
        MOQT_CORE_EXPORT int b() {}

        enum Enum2 {
            Val11 = 1,
            Val12 = 2,
            Val13 = 3,
        };
    };

}
