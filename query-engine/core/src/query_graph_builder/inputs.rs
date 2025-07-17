use std::slice;

use query_structure::{Filter, SelectionResult, WriteArgs};

use crate::{Computation, Flow, Node, NodeInputField, Query, ReadQuery, WriteQuery};

#[derive(Debug)]
pub(crate) struct UpdateOrCreateArgsInput;

impl NodeInputField<[WriteArgs]> for UpdateOrCreateArgsInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut [WriteArgs] {
        if let Node::Query(Query::Write(wn)) = node {
            match wn {
                WriteQuery::UpdateRecord(ur) => slice::from_mut(ur.args_mut()),
                WriteQuery::UpdateManyRecords(urm) => slice::from_mut(&mut urm.args),
                WriteQuery::CreateRecord(cr) => slice::from_mut(&mut cr.args),
                WriteQuery::CreateManyRecords(cr) => &mut cr.args,
                _ => panic!("UpdateOrCreateArgsInput can only be used with update or create nodes",),
            }
        } else {
            panic!("UpdateOrCreateArgsInput can only be used with WriteQuery nodes")
        }
    }
}

#[derive(Debug)]
pub(crate) struct RecordQueryFilterInput;

impl NodeInputField<Filter> for RecordQueryFilterInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut Filter {
        if let Node::Query(Query::Read(ReadQuery::RecordQuery(rq))) = node {
            rq.filter.get_or_insert(Filter::empty())
        } else {
            panic!("RecordQueryFilterInput can only be used with RecordQuery node")
        }
    }
}

#[derive(Debug)]
pub(crate) struct UpdateRecordSelectorsInput;

impl NodeInputField<Vec<SelectionResult>> for UpdateRecordSelectorsInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ur))) = node {
            ur.record_filter_mut().selectors.get_or_insert_default()
        } else {
            panic!("UpdateRecordFilterInput can only be used with UpdateRecord node")
        }
    }
}

#[derive(Debug)]
pub(crate) struct UpdateManyRecordsSelectorsInput;

impl NodeInputField<Vec<SelectionResult>> for UpdateManyRecordsSelectorsInput {
    fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ur))) = node {
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

#[derive(Debug)]
pub(crate) struct IfInput;

impl NodeInputField<Vec<SelectionResult>> for IfInput {
    fn node_input_field<'a>(&self, node: &'a mut Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Flow(Flow::If { data, .. }) = node {
            data
        } else {
            panic!("IfInput can only be used with If node")
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReturnInput;

impl NodeInputField<Vec<SelectionResult>> for ReturnInput {
    fn node_input_field<'a>(&self, node: &'a mut Node) -> &'a mut Vec<SelectionResult> {
        if let Node::Flow(Flow::Return(data)) = node {
            data
        } else {
            panic!("ReturnInput can only be used with Return node")
        }
    }
}
