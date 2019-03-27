use cpp_utils::{ConstPtr, DynamicCast, Ptr, StaticUpcast};
use qt_core::{
    ItemDataRole, QAbstractItemModel, QAbstractListModel, QAbstractTableModel, QString,
    QStringList, QStringListModel,
};

#[test]
fn models_and_casts() {
    unsafe {
        let mut string_list = QStringList::new();
        string_list.append(QString::from_std_str("text1").as_ptr());
        string_list.append(QString::from_std_str("text2").as_ptr());
        let mut string_list_model = QStringListModel::new4(string_list.as_ptr());
        assert_eq!(string_list_model.row_count_0a(), 2);
        {
            let index = string_list_model.index_2a(0, 0);
            assert_eq!(
                string_list_model
                    .data_2a(index.as_ptr(), ItemDataRole::DisplayRole.to_int())
                    .to_string()
                    .to_std_string(),
                "text1"
            );
        }
        {
            let index = string_list_model.index_2a(1, 0);
            assert_eq!(
                string_list_model
                    .data_2a(index.as_ptr(), ItemDataRole::DisplayRole.to_int())
                    .to_string()
                    .to_std_string(),
                "text2"
            );
        }

        let mut abstract_model: Ptr<QAbstractListModel> = string_list_model.static_upcast_mut();
        let abstract_model2: Ptr<QAbstractItemModel> = abstract_model.static_upcast_mut();
        assert_eq!(abstract_model.row_count_0a(), 2);
        {
            let string_list_model_back: Ptr<QStringListModel> = abstract_model
                .dynamic_cast_mut()
                .expect("dynamic_cast should be successful");
            assert_eq!(string_list_model_back.row_count_0a(), 2);
        }

        let table_model_attempt: Option<ConstPtr<QAbstractTableModel>> =
            abstract_model2.dynamic_cast();
        assert!(table_model_attempt.is_none());
    }
}
