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

macro_rules! node_input_field {
    ($name:ident,
     $type:ty,
     $variant:pat => $expr:expr
    ) => {
        #[derive(Debug)]
        pub(crate) struct $name;

        impl NodeInputField<$type> for $name {
            fn node_input_field<'a>(&self, node: &'a mut crate::Node) -> &'a mut $type {
                if let $variant = node {
                    $expr
                } else {
                    panic!(
                        "{}",
                        concat!(stringify!($name), " can only be used with ", stringify!($variant))
                    )
                }
            }
        }
    };
}

node_input_field!(
    RecordQueryFilterInput,
    Filter,
    Node::Query(Query::Read(ReadQuery::RecordQuery(rq))) => rq.filter.get_or_insert(Filter::empty())
);

node_input_field!(
    ManyRecordsQueryFilterInput,
    Filter,
    Node::Query(Query::Read(ReadQuery::ManyRecordsQuery(mrq))) => mrq.args.filter.get_or_insert(Filter::empty())
);

node_input_field!(
    UpdateRecordSelectorsInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::UpdateRecord(ur))) => ur.record_filter_mut().selectors.get_or_insert_default()
);

node_input_field!(
    UpdateManyRecordsSelectorsInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ur))) => ur.record_filter.selectors.get_or_insert_default()
);

node_input_field!(
    DeleteRecordSelectorsInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::DeleteRecord(dr))) => dr.record_filter.selectors.get_or_insert_default()
);

node_input_field!(
    DeleteManyRecordsSelectorsInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::DeleteManyRecords(dr))) => dr.record_filter.selectors.get_or_insert_default()
);

node_input_field!(
    LeftSideDiffInput,
    Vec<SelectionResult>,
    Node::Computation(Computation::DiffLeftToRight(diff_node) | Computation::DiffRightToLeft(diff_node)) => &mut diff_node.left
);

node_input_field!(
    RightSideDiffInput,
    Vec<SelectionResult>,
    Node::Computation(Computation::DiffLeftToRight(diff_node) | Computation::DiffRightToLeft(diff_node)) => &mut diff_node.right
);

node_input_field!(
    IfInput,
    Vec<SelectionResult>,
    Node::Flow(Flow::If { data, .. }) => data
);

node_input_field!(
    ReturnInput,
    Vec<SelectionResult>,
    Node::Flow(Flow::Return(data)) => data
);

node_input_field!(
    RelatedRecordsSelectorsInput,
    Vec<SelectionResult>,
    Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(rq))) => rq.parent_results.get_or_insert_default()
);

node_input_field!(
    ConnectParentInput,
    Option<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::ConnectRecords(cr))) => &mut cr.parent_id
);

node_input_field!(
    ConnectChildrenInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::ConnectRecords(cr))) => &mut cr.child_ids
);

node_input_field!(
    DisconnectParentInput,
    Option<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::DisconnectRecords(dr))) => &mut dr.parent_id
);

node_input_field!(
    DisconnectChildrenInput,
    Vec<SelectionResult>,
    Node::Query(Query::Write(WriteQuery::DisconnectRecords(dr))) => &mut dr.child_ids
);
