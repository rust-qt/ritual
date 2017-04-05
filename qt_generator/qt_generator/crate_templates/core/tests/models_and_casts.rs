extern crate qt_core;
use qt_core::cpp_utils::*;
use qt_core::string::String;
use qt_core::string_list::StringList;
use qt_core::string_list_model::StringListModel;
use qt_core::abstract_item_model::AbstractItemModel;
use qt_core::abstract_table_model::AbstractTableModel;
use qt_core::qt::ItemDataRole;

#[test]
fn models_and_casts() {
  let mut string_list = StringList::new(());
  string_list.append(&String::from("text1"));
  string_list.append(&String::from("text2"));
  let mut string_list_model = StringListModel::new(&string_list);
  assert_eq!(string_list_model.row_count(()), 2);
  {
    let index = string_list_model.index((0, 0));
    assert_eq!(string_list_model
                 .data(&index, ItemDataRole::Display as i32)
                 .to_string()
                 .to_std_string(),
               "text1");
  }
  {
    let index = string_list_model.index((1, 0));
    assert_eq!(string_list_model
                 .data(&index, ItemDataRole::Display as i32)
                 .to_string()
                 .to_std_string(),
               "text2");
  }

  let abstract_model: &mut AbstractItemModel = string_list_model.static_cast_mut();
  assert_eq!(abstract_model.row_count(()), 2);
  {
    let string_list_model_back: &mut StringListModel =
      abstract_model
        .dynamic_cast_mut()
        .expect("dynamic_cast should be successful");
    assert_eq!(string_list_model_back.row_count(()), 2);
  }

  let table_model_attempt: Option<&AbstractTableModel> = abstract_model.dynamic_cast();
  assert!(table_model_attempt.is_none());
}
