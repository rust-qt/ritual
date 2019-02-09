#include "moqt_core_exports.h"

namespace ns1 {
    MOQT_CORE_EXPORT int x() {
        return 1;
    }

    namespace ns2 {
        MOQT_CORE_EXPORT int x() {
            return 2;
        }
        MOQT_CORE_EXPORT int y() {
            return 3;
        }

        enum Enum1 {
            Val1,
            Val2,
            Val3
        };
    };

    namespace ns3 {
        MOQT_CORE_EXPORT int a() {
            return 4;
        }
        MOQT_CORE_EXPORT int b() {
            return 5;
        }

        enum Enum2 {
            Val11 = 1,
            Val12 = 2,
            Val13 = 3,
        };
    };

}
