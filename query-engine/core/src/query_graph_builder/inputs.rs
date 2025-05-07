use query_structure::SelectionResult;

use crate::{Computation, Node, NodeInputField, Query, WriteQuery};

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

#[derive(Debug)]
pub(crate) struct UpdateManyRecordsFilterInput;

impl NodeInputField<Vec<SelectionResult>> for UpdateManyRecordsFilterInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = node {
            ur.record_filter.selectors.get_or_insert_default()
        } else {
            panic!("UpdateManyRecordsFilterInput can only be used with UpdateManyRecords node")
        }
    }
}

#[derive(Debug)]
pub(crate) struct LeftSideDiffInput;

impl NodeInputField<Vec<SelectionResult>> for LeftSideDiffInput {
    fn node_input_field<'a>(&self, node: &'a mut Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Computation(Computation::DiffLeftToRight(diff_node) | Computation::DiffRightToLeft(diff_node)) =
            node
        {
            &mut diff_node.left
        } else {
            panic!("LeftSideDiffInput can only be used with DiffLeftToRight or DiffRightToLeft node")
        }
    }
}

#[derive(Debug)]
pub(crate) struct RightSideDiffInput;

impl NodeInputField<Vec<SelectionResult>> for RightSideDiffInput {
    fn node_input_field<'a>(&self, node: &'a mut Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Computation(Computation::DiffLeftToRight(diff_node) | Computation::DiffRightToLeft(diff_node)) =
            node
        {
            &mut diff_node.right
        } else {
            panic!("RightSideDiffInput can only be used with DiffLeftToRight or DiffRightToLeft node")
        }
    }
}
