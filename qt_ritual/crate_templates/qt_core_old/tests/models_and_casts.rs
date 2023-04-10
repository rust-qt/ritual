use qt_core::{
    ItemDataRole, QAbstractItemModel, QAbstractTableModel, QString, QStringList, QStringListModel,
};

#[test]
fn models_and_casts() {
    unsafe {
        let string_list = QStringList::new();
        string_list.append_q_string(&QString::from_std_str("text1"));
        string_list.append_q_string(&QString::from_std_str("text2"));
        let string_list_model = QStringListModel::from_q_string_list(&string_list);
        assert_eq!(string_list_model.row_count_0a(), 2);

        let index0 = string_list_model.index_2a(0, 0);
        assert_eq!(
            string_list_model
                .data_2a(&index0, ItemDataRole::DisplayRole.to_int())
                .to_string()
                .to_std_string(),
            "text1"
        );

        let index1 = string_list_model.index_2a(1, 0);
        assert_eq!(
            string_list_model
                .data_2a(&index1, ItemDataRole::DisplayRole.to_int())
                .to_string()
                .to_std_string(),
            "text2"
        );

        let abstract_model = string_list_model.static_upcast::<QAbstractItemModel>();
        assert_eq!(abstract_model.row_count_0a(), 2);

        let string_list_model_back = abstract_model.dynamic_cast::<QStringListModel>();
        assert!(
            !string_list_model_back.is_null(),
            "dynamic_cast should be successful"
        );
        assert_eq!(string_list_model_back.row_count_0a(), 2);

        assert!(abstract_model
            .dynamic_cast::<QAbstractTableModel>()
            .is_null());
    }
}
