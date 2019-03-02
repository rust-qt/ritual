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