use crate::cpp_operator::*;

#[test]
fn info1() {
    let info = CppOperator::Modulo.info();
    assert_eq!(info.function_name_suffix.unwrap(), "%");
    assert_eq!(info.arguments_count, 2);
    assert_eq!(info.allows_variadic_arguments, false);
}
