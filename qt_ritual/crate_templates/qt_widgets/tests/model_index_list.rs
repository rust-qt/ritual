use qt_widgets::{QApplication, QListView, SlotOfQListOfQModelIndex};

#[test]
fn model_index_list_slot() {
    QApplication::init(|_| unsafe {
        let mut list = QListView::new_0a();
        let slot = SlotOfQListOfQModelIndex::new(|x| {});
        let c = list.indexes_moved().connect(&slot);
        assert!(c.is_valid());
        0
    })
}
