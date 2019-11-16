#include "namespaces.h"

namespace ns1 {
    int x() {
        return 1;
    }

    namespace ns2 {
        int x() {
            return 2;
        }
        int y() {
            return 3;
        }
    }

    namespace ns3 {
        int a() {
            return 4;
        }
        int b() {
            return 5;
        }
    }
}

ns1::Templated1<int> func1() {
    return ns1::Templated1<int>();
}

ns1::ClassNs::Templated2<bool> func2() {
    return ns1::ClassNs::Templated2<bool>();
}

ignored_ns::Templated3<int> func3() {
    return ignored_ns::Templated3<int>();
}
