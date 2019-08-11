use crate::doc_parser::DocForType;
use ritual::cpp_data::CppPath;
use ritual::database::DocItem;
use ritual_common::errors::Result;

#[derive(Debug)]
pub struct MockDocParser;

impl MockDocParser {
    pub fn doc_for_method(
        &mut self,
        name: &str,
        _declaration1: &str,
        _declaration2: &str,
    ) -> Result<DocItem> {
        Ok(DocItem {
            anchor: None,
            html: format!(
                "<p>Fake doc for method: {}</p>\n\n<p>Some extra helpful notes.</p>",
                name
            ),
            mismatched_declaration: None,
            url: None,
            cross_references: vec![],
        })
    }

    pub fn doc_for_type(&mut self, path: &CppPath) -> Result<DocForType> {
        Ok(DocForType {
            type_doc: DocItem {
                anchor: None,
                html: format!(
                    "<p>Fake doc for type: {}</p>\n\n<p>Some extra helpful notes.</p>",
                    path.to_cpp_pseudo_code()
                ),
                mismatched_declaration: None,
                url: None,
                cross_references: vec![],
            },
            enum_variants_doc: Vec::new(),
        })
    }
}
