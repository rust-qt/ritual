use cpp_core::NullPtr;
use qt_widgets::{QApplication, QListView, SlotOfQListOfQModelIndex};

#[test]
fn model_index_list_slot() {
    QApplication::init(|_| unsafe {
        let list = QListView::new_0a();
        let slot = SlotOfQListOfQModelIndex::new(NullPtr, |_| {});
        let c = list.indexes_moved().connect(&slot);
        assert!(c.is_valid());
        0
    })
}
