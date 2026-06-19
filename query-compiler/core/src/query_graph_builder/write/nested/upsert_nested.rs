use super::*;
use crate::inputs::{
    IfInput, ReturnInput, UpdateManyRecordsSelectorsInput, UpdateOrCreateArgsInput, UpdateRecordSelectorsInput,
};
use crate::query_graph_builder::write::utils::coerce_values;
use crate::query_graph_builder::write::write_args_parser::WriteArgsParser;
use crate::{DataExpectation, RowSink};
use crate::{
    ParsedInputMap, ParsedInputValue,
    query_graph::{Flow, NodeRef, QueryGraph, QueryGraphDependency},
};
use query_structure::{Filter, Model, RelationFieldRef};
use schema::constants::{args, operations};
use std::{borrow::Cow, convert::TryInto};

/// Handles a nested upsert.
/// The constructed query graph can have different shapes based on the relation
/// of parent and child and where relations are inlined:
///
/// Many-to-many relation:
/// ```text
///                           ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///                                 Parent       ───────────────────┐
///                           └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │         │
///                                    │                            │
///                                    │                  │         │
///                                    ▼                            │
///                           ┌─────────────────┐         │         │
/// ┌───────────┬─────────────│   Read Child    │                   │
/// │           │             └─────────────────┘         │         │
/// │           │                      │                  ▼         │
/// │           │                      │           ┌ ─ ─ ─ ─ ─ ─    │
/// │           │                      │               Result   │   │
/// │           │                      │           └ ─ ─ ─ ─ ─ ─    │
/// │           │                      ▼                            │
/// │           │             ┌─────────────────┐                   │
/// │           │             │   If (exists)   │───────┐           │
/// │ ┌ ─ ─ ─ ─ ▼ ─ ─ ─ ─ ┐   └─────────────────┘       │           │
/// │  ┌─────────────────┐             │                │           │
/// │ ││    Join Node    │◀────Then────┘                │           │
/// │  └─────────────────┘                              │           │
/// │ │         │         │                             │           │
/// │           │                                       │           │
/// │ │         ▼         │                             │           │
/// │  ┌─────────────────┐                              ▼           │
/// │ ││ Insert onUpdate ││                    ┌─────────────────┐  │
/// │  │emulation subtree│                     │  Create Child   │  │
/// │ ││for all relations││                    └─────────────────┘  │
/// │  │ pointing to the │                              │           │
/// │ ││   Child model   ││                             │           │
/// │  └─────────────────┘                              │           │
/// │ └ ─ ─ ─ ─ ┬ ─ ─ ─ ─ ┘                             ▼           │
/// │           │                              ┌─────────────────┐  │
/// │           │                              │     Connect     │◀─┘
/// │           ▼                              └─────────────────┘
/// │  ┌─────────────────┐
/// └─▶│  Update Child   │
///    └─────────────────┘
/// ```
///
/// One-to-x relation:
/// ```text
///    Inlined in parent:                                                                      Inlined in child:
///                                                                                                                          ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///                                     ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐                                                                        Parent       ────────────────────┐
///                                           Parent       ────────┬──────────┐                                              └ ─ ─ ─ ─ ─ ─ ─ ─ ┘           │        │
///                                     └ ─ ─ ─ ─ ─ ─ ─ ─ ┘                   │                                                       │                             │
///                                              │                 │          │                                                       │                    │        │
///                                              │                            │                                                       │                             │
///                                              │                 │          │                                                       ▼                    ▼        │
///                                              ▼                 ▼          │                                              ┌─────────────────┐    ┌ ─ ─ ─ ─ ─ ─   │
///                                     ┌─────────────────┐ ┌ ─ ─ ─ ─ ─ ─     │                ┌─────────────────────────────│   Read Child    │        Result   │  │
/// ┌───────────────────────────────────│   Read Child    │     Result   │    │                │                             └─────────────────┘    └ ─ ─ ─ ─ ─ ─   │
/// │                                   └─────────────────┘ └ ─ ─ ─ ─ ─ ─     │                │                                      │                             │
/// │                                            │                            │                │                                      │                             │
/// │                                            │                            │                │ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐                ▼                             │
/// │                                            │                            │                │  ┌─────────────────┐        ┌─────────────────┐                    │
/// │ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐                      ▼                            │                │ ││    Join node    │◀───────│   If (exists)   │────────┐           │
/// │  ┌─────────────────┐              ┌─────────────────┐                   │                │  └─────────────────┘        └─────────────────┘        │           │
/// │ ││    Join node    │◀────Then─────│   If (exists)   │───────┐           │                │ │         │         │                                  │           │
/// │  └─────────────────┘              └─────────────────┘       │           │                │           ▼                                            │           │
/// │ │         │         │                                       │           │                │ │┌─────────────────┐│                                  │           │
/// │           ▼                                                 │           │                │  │ Insert onUpdate │                                   │           │
/// │ │┌─────────────────┐│                                       │           │                │ ││emulation subtree││                                  │           │
/// │  │ Insert onUpdate │                                        ▼           │                │  │for all relations│                                   ▼           │
/// │ ││emulation subtree││                              ┌─────────────────┐  │                │ ││ pointing to the ││                         ┌─────────────────┐  │
/// │  │for all relations│                               │  Create Child   │  │                │  │   Child model   │                          │  Create Child   │◀─┘
/// │ ││ pointing to the ││                              └─────────────────┘  │                │ │└─────────────────┘│                         └─────────────────┘
/// │  │   Child model   │                                        │           │                │  ─ ─ ─ ─ ─│─ ─ ─ ─ ─
/// │ │└─────────────────┘│                                       │           │                │           │
/// │  ─ ─ ─ ─ ─│─ ─ ─ ─ ─                                        │           │                │           │
/// │           │                                                 ▼           │                │           ▼
/// │           ▼                                        ┌─────────────────┐  │                │  ┌─────────────────┐
/// │  ┌─────────────────┐                               │  Update Parent  │◀─┘                └─▶│  Update Child   │
/// └─▶│  Update Child   │                               └─────────────────┘                      └─────────────────┘
///    └─────────────────┘
/// ```
/// Todo split this mess up and clean up the code.
pub fn nested_upsert(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let child_model_identifier = child_model.shard_aware_primary_identifier();

    for value in coerce_values(value) {
        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        let mut as_map: ParsedInputMap<'_> = value.try_into()?;
        let create_input = as_map.swap_remove(args::CREATE).expect("create argument is missing");
        let update_input = as_map.swap_remove(args::UPDATE).expect("update argument is missing");
        let where_input = as_map.swap_remove(args::WHERE);
        let mut create_map: ParsedInputMap<'_> = create_input.try_into()?;
        let mut update_map: ParsedInputMap<'_> = update_input.try_into()?;
        let shared_connect = extract_common_m2m_connect(&child_model, &mut create_map, &mut update_map);

        // Read child(ren) node
        let filter = match (where_input, parent_relation_field.is_list()) {
            // On a to-many relation the filter is a WhereUniqueInput
            (Some(where_input), true) => {
                let where_input: ParsedInputMap<'_> = where_input.try_into()?;

                extract_unique_filter(where_input, &child_model)?
            }
            // That filter is required. This should be caught by the schema validation.
            (None, true) => unreachable!("where argument is missing"),
            // On a to-one relation, the filter is a WhereInput (because the record is pinned by the QE automatically)
            (Some(where_input), false) => {
                let where_input: ParsedInputMap<'_> = where_input.try_into()?;

                extract_filter(where_input, &child_model)?
            }
            // That filter is optional since the record is pinned by the QE
            (None, false) => Filter::empty(),
        };

        let read_children_node =
            utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, filter.clone())?;

        let if_node = graph.create_node(Flow::if_non_empty());
        let create_node = create::create_record_node(graph, query_schema, child_model.clone(), create_map)?;
        let update_args = WriteArgsParser::from(&child_model, update_map)?;
        let is_nested_only_update = update_args.args.is_empty() && !update_args.nested.is_empty();
        let is_shared_connect_only_update =
            shared_connect.is_some() && update_args.args.is_empty() && update_args.nested.is_empty();

        graph.create_edge(
            &read_children_node,
            &if_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                RowSink::ProjectedPlaceholder(&IfInput),
                None,
            ),
        )?;

        let then_node = if is_nested_only_update || is_shared_connect_only_update {
            let return_node = graph.create_node(Flow::Return(None));

            graph.create_edge(
                &read_children_node,
                &return_node,
                QueryGraphDependency::ProjectedDataDependency(
                    child_model_identifier.clone(),
                    RowSink::ProjectedPlaceholder(&ReturnInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(&child_model)
                            .relation(&parent_relation_field.relation())
                            .needed_for(DependentOperation::nested_update())
                            .operation(DataOperation::NestedUpsert)
                            .build(),
                    )),
                ),
            )?;

            for (relation_field, data_map) in update_args.nested {
                connect_nested_query(graph, query_schema, return_node, relation_field, data_map)?;
            }

            return_node
        } else {
            let update_node = update::update_record_node_from_args(
                graph,
                query_schema,
                filter.clone(),
                child_model.clone(),
                update_args,
                None,
            )?;

            graph.create_edge(
                &read_children_node,
                &update_node,
                QueryGraphDependency::ProjectedDataDependency(
                    child_model_identifier.clone(),
                    RowSink::ExactlyOne(&UpdateRecordSelectorsInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(&child_model)
                            .relation(&parent_relation_field.relation())
                            .needed_for(DependentOperation::nested_update())
                            .operation(DataOperation::NestedUpsert)
                            .build(),
                    )),
                ),
            )?;

            // In case the connector doesn't support referential integrity, we add a subtree to the graph that emulates the ON_UPDATE referential action.
            // When that's the case, we create an intermediary node to which we connect all the nodes reponsible for emulating the referential action
            // Then, we connect the if node to that intermediary emulation node. This enables performing the emulation only in case the graph traverses
            // the update path (if the children already exists and goes to the THEN node).
            // It's only after we've executed the emulation that it'll traverse the update node, hence the ExecutionOrder between
            // the emulation node and the update node.
            if let Some(emulation_node) = utils::insert_emulated_on_update_with_intermediary_node(
                graph,
                query_schema,
                &child_model,
                &read_children_node,
                &update_node,
            )? {
                graph.create_edge(&emulation_node, &update_node, QueryGraphDependency::ExecutionOrder)?;

                emulation_node
            } else {
                update_node
            }
        };

        graph.create_edge(&if_node, &then_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

        // Specific handling based on relation type and inlining side.
        if parent_relation_field.relation().is_many_to_many() {
            // Many to many only needs a connect node.
            connect::connect_records_node(graph, &parent_node, &create_node, parent_relation_field, 1)?;
        } else if parent_relation_field.is_inlined_on_enclosing_model() {
            let parent_model = parent_relation_field.model();
            let parent_model_id = parent_model.shard_aware_primary_identifier();
            let update_node = utils::update_records_node_placeholder(graph, filter, parent_model.clone());

            // Edge to retrieve the finder
            graph.create_edge(
                &parent_node,
                &update_node,
                QueryGraphDependency::ProjectedDataDependency(
                    parent_model_id,
                    RowSink::ExactlyOne(&UpdateManyRecordsSelectorsInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(&parent_model)
                            .relation(&parent_relation_field.relation())
                            .needed_for(DependentOperation::update_inlined_relation(&parent_model))
                            .operation(DataOperation::NestedUpsert)
                            .build(),
                    )),
                ),
            )?;

            // Edge to retrieve the child ID to inject
            graph.create_edge(
                &create_node,
                &update_node,
                QueryGraphDependency::ProjectedDataDependency(
                    child_link.clone(),
                    RowSink::ExactlyOneWriteArgs(parent_link, &UpdateOrCreateArgsInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(&parent_relation_field.related_model())
                            .relation(&parent_relation_field.relation())
                            .needed_for(DependentOperation::update_inlined_relation(
                                &parent_relation_field.model(),
                            ))
                            .operation(DataOperation::NestedUpsert)
                            .build(),
                    )),
                ),
            )?;
        } else {
            // Inlined on child
            // Edge to retrieve the child ID to inject (inject into the create)
            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ProjectedDataDependency(
                    parent_link,
                    RowSink::ExactlyOneWriteArgs(child_link, &UpdateOrCreateArgsInput),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(&parent_relation_field.model())
                            .relation(&parent_relation_field.relation())
                            .needed_for(DependentOperation::update_inlined_relation(
                                &parent_relation_field.related_model(),
                            ))
                            .operation(DataOperation::NestedUpsert)
                            .build(),
                    )),
                ),
            )?;
        }

        if let Some((relation_field, connect_value)) = shared_connect {
            connect_nested_query(
                graph,
                query_schema,
                if_node,
                relation_field,
                [(Cow::Borrowed(operations::CONNECT), connect_value)]
                    .into_iter()
                    .collect(),
            )?;
        }
    }

    Ok(())
}

fn extract_common_m2m_connect<'a>(
    child_model: &Model,
    create_map: &mut ParsedInputMap<'a>,
    update_map: &mut ParsedInputMap<'a>,
) -> Option<(RelationFieldRef, ParsedInputValue<'a>)> {
    if update_map.len() != 1 {
        return None;
    }

    let (create_field_name, create_relation_field, create_connect) = single_m2m_connect(child_model, create_map)?;
    let (update_field_name, _, update_connect) = single_m2m_connect(child_model, update_map)?;

    if create_field_name != update_field_name || create_connect != update_connect {
        return None;
    }

    create_map.shift_remove(create_field_name.as_ref())?;
    update_map.shift_remove(update_field_name.as_ref())?;

    Some((create_relation_field, update_connect))
}

fn single_m2m_connect<'a>(
    model: &Model,
    map: &ParsedInputMap<'a>,
) -> Option<(Cow<'a, str>, RelationFieldRef, ParsedInputValue<'a>)> {
    let mut connect = None;

    for (field_name, value) in map.iter() {
        let Ok(relation_field) = model.fields().find_from_relation_fields(field_name) else {
            continue;
        };

        if connect.is_some() || !relation_field.relation().is_many_to_many() {
            return None;
        }

        let ParsedInputValue::Map(envelope) = value else {
            return None;
        };

        if envelope.len() != 1 {
            return None;
        }

        let connect_value = envelope.get(operations::CONNECT)?.clone();
        connect = Some((field_name.clone(), relation_field, connect_value));
    }

    connect
}
