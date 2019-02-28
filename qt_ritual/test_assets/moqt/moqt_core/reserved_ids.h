#include "moqt_core_exports.h"

enum impl {
    trait,
    use,
    crate = trait + 1,
    last = -1,
};

class MOQT_CORE_EXPORT unsafe {
public:
    int loop() { return 1; }
    void yield(int as) {}
    unsafe pub() {
        return unsafe();
    }

    float super;
};

namespace self {
    MOQT_CORE_EXPORT void box(int a);
}
