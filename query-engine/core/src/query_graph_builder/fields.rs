use query_structure::SelectionResult;

use crate::{Node, NodeInputField, Query, WriteQuery};

#[derive(Debug)]
pub(crate) struct UpdateRecordFilterInput;

impl NodeInputField<Vec<SelectionResult>> for UpdateRecordFilterInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = node {
            ur.record_filter_mut().selectors.get_or_insert_default()
        } else {
            panic!("UpdateRecordFilterInput can only be used with UpdateRecord node")
        }
    }
}
