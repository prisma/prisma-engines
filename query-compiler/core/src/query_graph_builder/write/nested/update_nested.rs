use super::*;
use crate::inputs::{ReturnInput, UpdateManyRecordsSelectorsInput, UpdateRecordSelectorsInput};
use crate::query_graph_builder::write::update::UpdateManyRecordNodeOptionals;
use crate::query_graph_builder::write::write_args_parser::WriteArgsParser;
use crate::{DataExpectation, RowSink};
use crate::{
    ParsedInputValue,
    query_graph::{Flow, NodeRef, QueryGraph, QueryGraphDependency},
};
use query_structure::{FieldSelection, Filter, Model, RelationFieldRef, SelectedField};
use schema::constants::args;
use std::convert::TryInto;

/// Handles nested update (single record) cases.
///
/// ```text
///       ┌ ─ ─ ─ ─ ─ ─
/// ┌─────    Parent   │─ ─ ─ ─ ─ ┐
/// │     └ ─ ─ ─ ─ ─ ─
/// │            │                │
/// │            ▼                ▼
/// │     ┌────────────┐    ┌ ─ ─ ─ ─ ─
/// │     │   Check    │       Result  │
/// │     └────────────┘    └ ─ ─ ─ ─ ─
/// │            │
/// │  ┌ ─ ─ ─ ─ ▼ ─ ─ ─ ─ ┐
/// │   ┌─────────────────┐
/// │  ││ Insert onUpdate ││
/// │   │emulation subtree│
/// │  ││for all relations││
/// │   │ pointing to the │
/// │  ││   Child model   ││
/// │   └─────────────────┘
/// │  └ ─ ─ ─ ─ ┬ ─ ─ ─ ─ ┘
/// │         ┌──┘
/// │         │
/// │         ▼
/// │  ┌────────────┐
/// └─▶│   Update   │
///    └────────────┘
/// ```
pub fn nested_update(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_values(value) {
        let (data, filter) = if parent_relation_field.is_list() {
            // We have to have a single record filter in "where".
            // This is used to read the children first, to make sure they're actually connected.
            // The update itself operates on the record found by the read check.
            let mut map: ParsedInputMap<'_> = value.try_into()?;
            let where_arg: ParsedInputMap<'_> = map.swap_remove(args::WHERE).unwrap().try_into()?;

            let filter = extract_unique_filter(where_arg, child_model)?;
            let data_value = map.swap_remove(args::DATA).unwrap();

            (data_value, filter)
        } else {
            match value {
                // If the update input is of shape { where?: WhereInput, data: DataInput }
                ParsedInputValue::Map(mut map) if map.is_nested_to_one_update_envelope() => {
                    let filter = if let Some(where_arg) = map.swap_remove(args::WHERE) {
                        let where_arg: ParsedInputMap<'_> = where_arg.try_into()?;

                        extract_filter(where_arg, child_model)?
                    } else {
                        Filter::empty()
                    };

                    let data_value = map.swap_remove(args::DATA).unwrap();

                    (data_value, filter)
                }
                // If the update input is the shorthand shape which directly updates data
                x => (x, Filter::empty()),
            }
        };

        let data_map: ParsedInputMap<'_> = data.try_into()?;

        // If there's nothing to update, skip the update entirely.
        if data_map.is_empty() {
            return Ok(());
        }

        let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();
        let update_args = WriteArgsParser::from(child_model, data_map)?;

        if update_args.args.is_empty() && !update_args.nested.is_empty() {
            if let Some(return_node) = insert_parent_link_return_node(
                graph,
                query_schema,
                parent,
                parent_relation_field,
                &filter,
                &child_model_identifier,
            )? {
                for (relation_field, data_map) in update_args.nested {
                    connect_nested_query(graph, query_schema, return_node, relation_field, data_map)?;
                }

                continue;
            }

            let find_child_records_node =
                utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter)?;
            let return_node = graph.create_node(Flow::Return(None));

            graph.create_edge(
                &find_child_records_node,
                &return_node,
                QueryGraphDependency::ProjectedDataDependency(
                    child_model_identifier,
                    RowSink::ProjectedPlaceholder(&ReturnInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(child_model)
                            .relation(&parent_relation_field.relation())
                            .operation(DataOperation::NestedUpdate)
                            .build(),
                    )),
                ),
            )?;

            for (relation_field, data_map) in update_args.nested {
                connect_nested_query(graph, query_schema, return_node, relation_field, data_map)?;
            }

            continue;
        }

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;
        let update_node =
            update::update_record_node_from_args(graph, query_schema, filter, child_model.clone(), update_args, None)?;

        graph.create_edge(
            &find_child_records_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                RowSink::ExactlyOne(&UpdateRecordSelectorsInput),
                Some(DataExpectation::non_empty_rows(
                    MissingRelatedRecord::builder()
                        .model(child_model)
                        .relation(&parent_relation_field.relation())
                        .operation(DataOperation::NestedUpdate)
                        .build(),
                )),
            ),
        )?;

        utils::insert_emulated_on_update(graph, query_schema, child_model, &find_child_records_node, &update_node)?;
    }

    Ok(())
}

fn insert_parent_link_return_node(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: &Filter,
    child_model_identifier: &FieldSelection,
) -> QueryGraphBuilderResult<Option<NodeRef>> {
    if query_schema.relation_mode().is_prisma()
        || parent_relation_field.is_list()
        || !parent_relation_field.is_required()
        || !parent_relation_field.is_inlined_on_enclosing_model()
        || !filter.is_empty()
    {
        return Ok(None);
    }

    let parent_scalars = parent_relation_field.scalar_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    if parent_scalars.len() != 1 || child_link.selections().len() != 1 || child_link != *child_model_identifier {
        return Ok(None);
    }

    let parent_link = FieldSelection::new(vec![SelectedField::Scalar(parent_scalars[0].clone())]);

    let return_node = graph.create_node(Flow::Return(None));

    graph.create_edge(
        parent,
        &return_node,
        QueryGraphDependency::ProjectedDataDependency(
            parent_link,
            RowSink::ProjectedFieldPlaceholder(&ReturnInput),
            None,
        ),
    )?;

    Ok(Some(return_node))
}

pub fn nested_update_many(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_values(value) {
        let mut map: ParsedInputMap<'_> = value.try_into()?;
        let where_arg = map.swap_remove(args::WHERE).unwrap();
        let data_value = map.swap_remove(args::DATA).unwrap();
        let data_map: ParsedInputMap<'_> = data_value.try_into()?;
        let where_map: ParsedInputMap<'_> = where_arg.try_into()?;
        let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();

        let filter = extract_filter(where_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter)?;

        let update_many_node = update::update_many_record_node(
            graph,
            query_schema,
            Filter::empty(),
            child_model.clone(),
            data_map,
            UpdateManyRecordNodeOptionals {
                name: None,
                nested_field_selection: None,
                limit: None,
            },
        )?;

        graph.create_edge(
            &find_child_records_node,
            &update_many_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                RowSink::All(&UpdateManyRecordsSelectorsInput),
                None,
            ),
        )?;
    }

    Ok(())
}
