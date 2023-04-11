use qt_gui::{QListOfQStandardItem, QStandardItem};

#[test]
fn list_of_pointers_append() {
    unsafe {
        let list = QListOfQStandardItem::new();

        let item = QStandardItem::new();
        item.set_enabled(true);

        list.append_q_standard_item(&item.as_mut_raw_ptr());

        let item2 = QStandardItem::new();
        item2.set_enabled(false);

        list.append_q_standard_item(&item2.as_mut_raw_ptr());

        assert!((**list.at(0)).is_enabled());
        assert!(!(**list.at(1)).is_enabled());
    }
}
