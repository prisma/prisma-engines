use crate::{
    TranslateError, binding,
    data_mapper::FieldType,
    expression::{
        Binding, EnumsMap, Expression, InMemoryOps, JoinExpression, RawNestedReadDirectRelation, RawNestedReadQuery,
        RawNestedReadRelation, RawResultColumnMapping, RawResultColumnRef, RawResultFieldName,
    },
    translate::TranslateResult,
};
use itertools::Itertools;
use query_builder::{ConditionalLink, DbQuery, QueryArgumentsExt, QueryBuilder, RelationLinkage};
use query_core::{
    AggregateRecordsQuery, DataExpectation, DataOperation, MissingRecord, QueryGraphBuilderError, QueryOption,
    QueryOptions, ReadQuery, RelatedRecordsQuery,
};
use query_structure::{
    ConditionValue, FieldSelection, Filter, Model, Placeholder, PrismaValue, QueryArguments, QueryMode, RelationField,
    RelationLoadStrategy, ScalarCondition, ScalarField, ScalarFilter, ScalarProjection, SelectedField, Take,
};
use std::{borrow::Cow, slice};

mod in_memory_processing;

pub(crate) fn translate_read_query(query: ReadQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(mut rq) => {
            let selected_fields = match rq.relation_load_strategy {
                RelationLoadStrategy::Join => rq.selected_fields.into_virtuals_last(),
                RelationLoadStrategy::Query => rq.selected_fields.into_without_relations().into_virtuals_last(),
            };

            let mut args = QueryArguments::from((
                rq.model.clone(),
                rq.filter.expect("ReadOne query should always have filter set"),
            ))
            .with_take(Take::One);

            let in_memory_ops =
                in_memory_processing::extract_in_memory_ops(&mut args, rq.relation_load_strategy, &mut rq.nested);

            if rq.relation_load_strategy == RelationLoadStrategy::Query
                && !rq.nested.is_empty()
                && in_memory_ops.is_empty()
                && !rq.options.contains(QueryOption::ThrowOnEmpty)
            {
                if let Some(expr) = build_raw_nested_read_root(
                    builder,
                    &rq.model,
                    args.clone(),
                    &selected_fields,
                    &rq.selection_order,
                    &rq.nested,
                    true,
                )? {
                    return Ok(expr);
                }
            }

            let expr = build_get_records(builder, &rq.model, args, &selected_fields, rq.relation_load_strategy)?;
            let expr = in_memory_ops.into_expression(expr);
            let expr = convert_options_to_validation(expr, rq.options);
            let expr = Expression::Unique(Box::new(expr));

            match rq.relation_load_strategy {
                RelationLoadStrategy::Query if !rq.nested.is_empty() => add_inmemory_join(expr, rq.nested, builder)?,
                _ => expr,
            }
        }

        ReadQuery::ManyRecordsQuery(mut mrq) => {
            // Skip the query entirely if the take is 0.
            if mrq.args.take == Take::Some(0) {
                return Ok(Expression::Concat(vec![]));
            }

            let selected_fields = match mrq.relation_load_strategy {
                RelationLoadStrategy::Join => mrq.selected_fields.into_virtuals_last(),
                RelationLoadStrategy::Query => mrq.selected_fields.into_without_relations().into_virtuals_last(),
            };

            let take = mrq.args.take;

            let in_memory_ops =
                in_memory_processing::extract_in_memory_ops(&mut mrq.args, mrq.relation_load_strategy, &mut mrq.nested);

            if mrq.relation_load_strategy == RelationLoadStrategy::Query
                && !mrq.nested.is_empty()
                && in_memory_ops.is_empty()
                && !mrq.options.contains(QueryOption::ThrowOnEmpty)
            {
                let unique = matches!(take, Take::One | Take::NegativeOne);
                if let Some(expr) = build_raw_nested_read_root(
                    builder,
                    &mrq.model,
                    mrq.args.clone(),
                    &selected_fields,
                    &mrq.selection_order,
                    &mrq.nested,
                    unique,
                )? {
                    return Ok(expr);
                }
            }

            let expr = build_get_records(
                builder,
                &mrq.model,
                mrq.args,
                &selected_fields,
                mrq.relation_load_strategy,
            )?;

            let expr = in_memory_ops.into_expression(expr);

            let mut expr = convert_options_to_validation(expr, mrq.options);

            if mrq.relation_load_strategy == RelationLoadStrategy::Query && !mrq.nested.is_empty() {
                expr = add_inmemory_join(expr, mrq.nested, builder)?;
            }

            match take {
                Take::One | Take::NegativeOne => Expression::Unique(Box::new(expr)),
                _ => expr,
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            let (expr, join) = build_read_related_records(rrq, vec![], false, builder)?;
            if join.is_relation_unique {
                Expression::Unique(Box::new(expr))
            } else {
                expr
            }
        }

        ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
            name: _,
            alias: _,
            // TODO: we're ignoring selection order
            selection_order: _,
            model,
            args,
            selectors,
            group_by,
            having,
        }) => {
            let has_group_by = !group_by.is_empty();
            let query = builder
                .build_aggregate(&model, args, &selectors, group_by, having)
                .map_err(TranslateError::QueryBuildFailure)?;
            let expr = Expression::Query(query);
            if has_group_by {
                expr
            } else {
                Expression::Unique(expr.into())
            }
        }
    })
}

pub(crate) fn guarantees_raw_nested_read_root(query: &ReadQuery) -> bool {
    match query {
        ReadQuery::RecordQuery(rq) => {
            rq.relation_load_strategy == RelationLoadStrategy::Query
                && !rq.nested.is_empty()
                && !rq.options.contains(QueryOption::ThrowOnEmpty)
                && rq.filter.as_ref().is_some_and(filter_cannot_chunk)
                && raw_result_mapping_supported(&rq.selected_fields, &rq.selection_order)
                && raw_nested_relations_supported(&rq.nested, &rq.selected_fields, true)
        }
        ReadQuery::ManyRecordsQuery(mrq) => {
            if mrq.args.take == Take::Some(0) {
                return false;
            }

            mrq.relation_load_strategy == RelationLoadStrategy::Query
                && !mrq.nested.is_empty()
                && root_in_memory_ops_empty(&mrq.args)
                && !mrq.options.contains(QueryOption::ThrowOnEmpty)
                && args_cannot_chunk(&mrq.args)
                && raw_result_mapping_supported(&mrq.selected_fields, &mrq.selection_order)
                && raw_nested_relations_supported(
                    &mrq.nested,
                    &mrq.selected_fields,
                    matches!(mrq.args.take, Take::One | Take::NegativeOne),
                )
        }
        ReadQuery::RelatedRecordsQuery(_) | ReadQuery::AggregateRecordsQuery(_) => false,
    }
}

fn root_in_memory_ops_empty(args: &QueryArguments) -> bool {
    !args.needs_reversed_order()
        && !args.requires_inmemory_pagination(RelationLoadStrategy::Query)
        && !args.requires_inmemory_distinct(RelationLoadStrategy::Query)
}

fn raw_nested_relations_supported(
    nested: &[ReadQuery],
    parent_selected_fields: &FieldSelection,
    has_unique_parent: bool,
) -> bool {
    nested.iter().all(|nested| {
        let ReadQuery::RelatedRecordsQuery(rrq) = nested else {
            return false;
        };

        let Some(parent_scalar) = rrq.parent_field.single_left_scalar() else {
            return false;
        };

        selected_fields_contain_db_name(parent_selected_fields, parent_scalar.db_name())
            && get_single_relation_scalar_for_filters(&rrq.parent_field).is_some()
            && raw_related_records_supported(rrq, has_unique_parent)
    })
}

fn raw_related_records_supported(rrq: &RelatedRecordsQuery, has_unique_parent: bool) -> bool {
    if rrq.args.take == Take::Some(0)
        || rrq.parent_results.is_some()
        || !args_cannot_chunk(&rrq.args)
        || !raw_nested_relation_operations_may_be_supported(&rrq.args, has_unique_parent)
        || !raw_result_mapping_supported(&rrq.selected_fields, &rrq.selection_order)
    {
        return false;
    }

    let is_many_to_many = rrq.parent_field.relation().is_many_to_many();
    let Some(child_scalar) = get_single_relation_scalar_for_filters(&rrq.parent_field) else {
        return false;
    };

    if !is_many_to_many && !selected_fields_contain_db_name(&rrq.selected_fields, child_scalar.db_name()) {
        return false;
    }

    let child_has_unique_parent = has_unique_parent && !is_many_to_many && !rrq.parent_field.arity().is_list();
    raw_nested_relations_supported(&rrq.nested, &rrq.selected_fields, child_has_unique_parent)
}

fn raw_nested_relation_operations_may_be_supported(args: &QueryArguments, has_unique_parent: bool) -> bool {
    if args.needs_reversed_order() {
        return false;
    }

    let needs_pagination = args.take.is_some() || args.skip.is_some() || args.cursor.is_some();
    let must_paginate_in_memory =
        needs_pagination && (!has_unique_parent || args.requires_inmemory_processing(RelationLoadStrategy::Query));
    if must_paginate_in_memory && args.cursor.is_some() {
        return false;
    }

    let needs_distinct = args.distinct.is_some();
    let must_distinct_in_memory =
        needs_distinct && (!has_unique_parent || args.requires_inmemory_distinct(RelationLoadStrategy::Query));

    !must_distinct_in_memory
}

fn args_cannot_chunk(args: &QueryArguments) -> bool {
    args.filter.as_ref().is_none_or(filter_cannot_chunk)
}

fn filter_cannot_chunk(filter: &Filter) -> bool {
    !filter.should_batch(1)
}

fn raw_result_mapping_supported(selected_fields: &FieldSelection, selection_order: &[String]) -> bool {
    if selected_fields
        .selections()
        .any(|field| matches!(field, SelectedField::Composite(_)))
    {
        return false;
    }

    selection_order.iter().all(|prisma_name| {
        let Some(selection) = selected_fields
            .selections()
            .filter(|field| !matches!(field, SelectedField::Relation(_)))
            .find(|field| field.prisma_name_grouping_virtuals() == prisma_name.as_str())
        else {
            return true;
        };

        selection.type_info().is_some()
    })
}

fn selected_fields_contain_db_name(selected_fields: &FieldSelection, db_name: &str) -> bool {
    selected_fields
        .selections()
        .filter(|field| !matches!(field, SelectedField::Relation(_)))
        .any(|field| field.db_name().as_ref() == db_name)
}

pub(super) fn add_inmemory_join(
    parent: Expression,
    nested: Vec<ReadQuery>,
    builder: &dyn QueryBuilder,
) -> TranslateResult<Expression> {
    let mut all_linking_fields = nested
        .iter()
        .flat_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => rrq.parent_field.left_scalars(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    all_linking_fields.sort_by(|a, b| a.name().cmp(b.name()));
    all_linking_fields.dedup_by(|a, b| a.name() == b.name());

    let linking_fields_bindings = all_linking_fields
        .iter()
        .map(|sf| Binding {
            name: binding::join_parent_field(sf),
            expr: Expression::MapField {
                field: sf.db_name().into(),
                records: Box::new(Expression::Get {
                    name: binding::join_parent(),
                }),
            },
        })
        .collect();

    let can_assume_strict_equality = nested
        .iter()
        .all(|nested| nested.model().dm.schema.connector.can_assume_strict_equality_in_joins());
    let join_expressions = nested
        .into_iter()
        .filter_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
            _ => None,
        })
        .map(|rrq| -> TranslateResult<JoinExpression> {
            let has_unique_parent = !parent.r#type().is_list();
            let prefixed_parent_field_name = binding::nested_relation_field(&rrq.parent_field);
            let left_scalars = rrq.parent_field.left_scalars();

            let links = left_scalars
                .iter()
                .zip(get_relation_scalars_for_filters(&rrq.parent_field))
                .map(|(parent_scalar, child_scalar)| {
                    let placeholder = Placeholder {
                        name: binding::join_parent_field(parent_scalar),
                        r#type: parent_scalar.type_info().to_prisma_type(),
                    };
                    let condition = if has_unique_parent {
                        ScalarCondition::Equals(ConditionValue::value(PrismaValue::from(placeholder)))
                    } else {
                        ScalarCondition::In(placeholder.into())
                    };
                    ConditionalLink::new(child_scalar.clone(), vec![condition])
                })
                .collect();
            let (child, join) = build_read_related_records(rrq, links, has_unique_parent, builder)?;

            Ok(JoinExpression {
                child,
                is_relation_unique: join.is_relation_unique,
                on: left_scalars
                    .into_iter()
                    .map(|sf| sf.db_name().into())
                    .zip(join.into_fields())
                    .collect(),
                parent_field: prefixed_parent_field_name.into_owned(),
            })
        })
        .try_collect()?;

    Ok(Expression::Let {
        bindings: vec![Binding {
            name: binding::join_parent(),
            expr: parent,
        }],
        expr: Box::new(Expression::Let {
            bindings: linking_fields_bindings,
            expr: Box::new(Expression::Join {
                parent: Box::new(Expression::Get {
                    name: binding::join_parent(),
                }),
                children: join_expressions,
                can_assume_strict_equality,
            }),
        }),
    })
}

struct BuiltRawNestedReadQuery {
    query: RawNestedReadQuery,
}

struct RawColumnIndexes<'a> {
    indexes: Vec<(Cow<'a, str>, usize)>,
}

impl RawColumnIndexes<'_> {
    fn get(&self, name: &str) -> Option<usize> {
        self.indexes
            .iter()
            .find_map(|(column, index)| (column.as_ref() == name).then_some(*index))
    }
}

fn build_raw_nested_read_root(
    builder: &dyn QueryBuilder,
    model: &Model,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    selection_order: &[String],
    nested: &[ReadQuery],
    unique: bool,
) -> TranslateResult<Option<Expression>> {
    let mut enums = EnumsMap::new();
    let Some(query) = build_raw_nested_read_query(
        builder,
        model,
        args,
        selected_fields,
        selection_order,
        nested,
        unique,
        &mut enums,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(Expression::RawNestedRead {
        query: query.query,
        unique,
        enums,
    }))
}

fn build_raw_nested_read_query(
    builder: &dyn QueryBuilder,
    model: &Model,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    selection_order: &[String],
    nested: &[ReadQuery],
    has_unique_parent: bool,
    enums: &mut EnumsMap,
) -> TranslateResult<Option<BuiltRawNestedReadQuery>> {
    let Some(db_query) = build_get_records_query(builder, model, args, selected_fields, RelationLoadStrategy::Query)?
    else {
        return Ok(None);
    };
    let column_indexes = raw_column_indexes(selected_fields);
    let Some(fields) = raw_result_column_mappings(selected_fields, selection_order, &column_indexes, enums) else {
        return Ok(None);
    };
    let Some(relations) = build_raw_nested_read_relations(nested, &column_indexes, has_unique_parent, builder, enums)?
    else {
        return Ok(None);
    };

    Ok(Some(BuiltRawNestedReadQuery {
        query: RawNestedReadQuery {
            query: db_query,
            fields,
            relations,
        },
    }))
}

fn build_get_records_query(
    builder: &dyn QueryBuilder,
    model: &Model,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    relation_load_strategy: RelationLoadStrategy,
) -> TranslateResult<Option<DbQuery>> {
    match build_get_records(builder, model, args, selected_fields, relation_load_strategy)? {
        Expression::Query(query) => Ok(Some(query)),
        _ => Ok(None),
    }
}

fn raw_column_indexes(selected_fields: &FieldSelection) -> RawColumnIndexes<'_> {
    RawColumnIndexes {
        indexes: selected_fields
            .selections()
            .enumerate()
            .map(|(index, field)| (field.db_name(), index))
            .collect(),
    }
}

fn raw_result_column_mappings(
    selected_fields: &FieldSelection,
    selection_order: &[String],
    column_indexes: &RawColumnIndexes<'_>,
    enums: &mut EnumsMap,
) -> Option<Vec<RawResultColumnMapping>> {
    let mut mappings = Vec::new();

    for prisma_name in selection_order {
        let Some(selection) = selected_fields
            .selections()
            .find(|field| field.prisma_name_grouping_virtuals() == prisma_name.as_str())
        else {
            continue;
        };

        match selection {
            SelectedField::Scalar(field) => {
                let column_index = column_indexes.get(field.db_name().as_ref())?;
                mappings.push(RawResultColumnMapping {
                    field_name: RawResultFieldName::Field(field.name().to_owned()),
                    column: RawResultColumnRef::Index(column_index),
                    field_type: Some(raw_field_type(selection, enums)?),
                });
            }
            SelectedField::Virtual(virtual_selection) => {
                for virtual_selection in selected_fields
                    .virtuals()
                    .filter(|field| field.serialized_group_name() == virtual_selection.serialized_group_name())
                {
                    let column_index = column_indexes.get(&virtual_selection.db_alias())?;
                    let (group_name, field_name) = virtual_selection.serialized_name();
                    mappings.push(RawResultColumnMapping {
                        field_name: RawResultFieldName::Path(vec![group_name.to_owned(), field_name.to_owned()]),
                        column: RawResultColumnRef::Index(column_index),
                        field_type: Some(raw_field_type(
                            &SelectedField::Virtual(virtual_selection.clone()),
                            enums,
                        )?),
                    });
                }
            }
            SelectedField::Relation(_) => {}
            SelectedField::Composite(_) => return None,
        }
    }

    Some(mappings)
}

fn raw_field_type(selection: &SelectedField, enums: &mut EnumsMap) -> Option<FieldType> {
    let type_info = selection.type_info()?;
    if let query_structure::TypeIdentifier::Enum(id) = type_info.typ.id {
        enums.add(type_info.typ.dm.clone().zip(id));
    }
    Some(FieldType::from(&type_info))
}

fn build_raw_nested_read_relations(
    nested: &[ReadQuery],
    parent_column_indexes: &RawColumnIndexes<'_>,
    has_unique_parent: bool,
    builder: &dyn QueryBuilder,
    enums: &mut EnumsMap,
) -> TranslateResult<Option<Vec<RawNestedReadRelation>>> {
    let mut relations = Vec::with_capacity(nested.len());

    for nested in nested {
        let ReadQuery::RelatedRecordsQuery(rrq) = nested else {
            return Ok(None);
        };

        let Some(parent_scalar) = rrq.parent_field.single_left_scalar() else {
            return Ok(None);
        };
        let Some(child_scalar) = get_single_relation_scalar_for_filters(&rrq.parent_field) else {
            return Ok(None);
        };
        let placeholder = Placeholder {
            name: binding::join_parent_field(&parent_scalar),
            r#type: parent_scalar.type_info().to_prisma_type(),
        };
        let condition = if has_unique_parent {
            ScalarCondition::Equals(ConditionValue::value(PrismaValue::from(placeholder)))
        } else {
            ScalarCondition::In(placeholder.into())
        };
        let links = vec![ConditionalLink::new(child_scalar, vec![condition])];

        let Some((child, join, child_column_index, operations)) =
            build_raw_read_related_records(rrq, links, has_unique_parent, builder, enums)?
        else {
            return Ok(None);
        };

        let Some(parent_column_index) = parent_column_indexes.get(parent_scalar.db_name().as_ref()) else {
            return Ok(None);
        };

        relations.push(RawNestedReadRelation::Direct(RawNestedReadDirectRelation {
            field_name: rrq.alias.as_deref().unwrap_or(&rrq.name).to_owned(),
            child: child.query,
            parent_column: RawResultColumnRef::Index(parent_column_index),
            child_column: RawResultColumnRef::Index(child_column_index),
            scope_name: binding::join_parent_field(&parent_scalar),
            is_relation_unique: join.is_relation_unique,
            operations,
        }));
    }

    Ok(Some(relations))
}

fn build_raw_read_related_records(
    rrq: &RelatedRecordsQuery,
    links: Vec<ConditionalLink>,
    has_unique_parent: bool,
    builder: &dyn QueryBuilder,
    enums: &mut EnumsMap,
) -> TranslateResult<Option<(BuiltRawNestedReadQuery, JoinMetadata, usize, InMemoryOps)>> {
    if rrq.args.take == Take::Some(0) {
        return Ok(None);
    }

    let is_many_to_many = rrq.parent_field.relation().is_many_to_many();
    let mut linkage = RelationLinkage::new(rrq.parent_field.clone(), links);

    if let Some(results) = rrq.parent_results.clone() {
        let parent_link_id = rrq.parent_field.linking_fields();
        let selection = results
            .into_iter()
            .exactly_one()
            .expect("parent results should be exactly one in the query compiler")
            .split_into(slice::from_ref(&parent_link_id))
            .pop()
            .unwrap();

        for (field, val) in FieldSelection::from(get_relation_scalars_for_filters(&rrq.parent_field))
            .assimilate(selection)
            .map_err(QueryGraphBuilderError::from)?
            .pairs
            .into_iter()
        {
            let Some(sf) = field.as_scalar() else { continue };
            let p = val.into_placeholder().expect("expected placeholder in parent results");
            linkage.add_condition(sf.clone(), ScalarCondition::In(p.into()));
        }
    }

    let selected_fields = rrq
        .selected_fields
        .clone()
        .into_without_relations()
        .into_virtuals_last();
    let mut args = rrq.args.clone();
    let in_memory_ops = in_memory_processing::extract_in_memory_ops_for_nested_query(&mut args, has_unique_parent);
    if !raw_nested_relation_operations_supported(&in_memory_ops) {
        return Ok(None);
    }

    let (child_query, join) = if is_many_to_many {
        build_read_m2m_query(linkage, args, &selected_fields, builder)?
    } else {
        build_read_one2m_query(linkage, args, &selected_fields, builder)?
    };
    let Expression::Query(db_query) = child_query else {
        return Ok(None);
    };
    let column_indexes = if is_many_to_many {
        let [linking_field_alias] = &join.fields[..] else {
            return Ok(None);
        };
        raw_many_to_many_child_column_indexes(&selected_fields, linking_field_alias.clone())
    } else {
        raw_column_indexes(&selected_fields)
    };
    let Some(fields) = raw_result_column_mappings(&selected_fields, &rrq.selection_order, &column_indexes, enums)
    else {
        return Ok(None);
    };
    let [child_field] = &join.fields[..] else {
        return Ok(None);
    };
    let Some(child_column_index) = column_indexes.get(child_field) else {
        return Ok(None);
    };
    let child_has_unique_parent = has_unique_parent && join.is_relation_unique;
    let Some(relations) =
        build_raw_nested_read_relations(&rrq.nested, &column_indexes, child_has_unique_parent, builder, enums)?
    else {
        return Ok(None);
    };

    Ok(Some((
        BuiltRawNestedReadQuery {
            query: RawNestedReadQuery {
                query: db_query,
                fields,
                relations,
            },
        },
        join,
        child_column_index,
        in_memory_ops,
    )))
}

fn raw_nested_relation_operations_supported(ops: &InMemoryOps) -> bool {
    ops.distinct.is_none()
        && !ops.reverse
        && ops.nested.is_empty()
        && ops.linking_fields.is_none()
        && ops
            .pagination
            .as_ref()
            .is_none_or(|pagination| pagination.cursor().is_none())
}

fn raw_many_to_many_child_column_indexes(
    selected_fields: &FieldSelection,
    linking_field_alias: String,
) -> RawColumnIndexes<'_> {
    let mut column_indexes = RawColumnIndexes {
        indexes: Vec::with_capacity(selected_fields.selections().len() + 1),
    };
    let mut next_index = 0;

    for field in selected_fields.selections() {
        if matches!(field, SelectedField::Scalar(_)) {
            column_indexes.indexes.push((field.db_name(), next_index));
            next_index += 1;
        }
    }

    // `build_get_related_records()` selects scalar model columns, then the hidden m2m linking alias,
    // then any additional virtual selections.
    column_indexes
        .indexes
        .push((Cow::Owned(linking_field_alias), next_index));
    next_index += 1;

    for field in selected_fields.selections() {
        if matches!(field, SelectedField::Virtual(_)) {
            column_indexes.indexes.push((field.db_name(), next_index));
            next_index += 1;
        }
    }

    column_indexes
}

fn build_read_related_records(
    mut rrq: RelatedRecordsQuery,
    links: Vec<ConditionalLink>,
    has_unique_parent: bool,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    // Skip the query entirely if the take is 0.
    if rrq.args.take == Take::Some(0) {
        return Ok((Expression::Concat(vec![]), JoinMetadata::default()));
    }

    let mut linkage = RelationLinkage::new(rrq.parent_field.clone(), links);

    if let Some(results) = rrq.parent_results {
        let parent_link_id = rrq.parent_field.linking_fields();
        let selection = results
            .into_iter()
            .exactly_one()
            .expect("parent results should be exactly one in the query compiler")
            .split_into(slice::from_ref(&parent_link_id))
            .pop()
            .unwrap();

        for (field, val) in FieldSelection::from(get_relation_scalars_for_filters(&rrq.parent_field))
            .assimilate(selection)
            .map_err(QueryGraphBuilderError::from)?
            .pairs
            .into_iter()
        {
            let Some(sf) = field.as_scalar() else { continue };
            let p = val.into_placeholder().expect("expected placeholder in parent results");
            linkage.add_condition(sf.clone(), ScalarCondition::In(p.into()));
        }
    }

    let selected_fields = rrq.selected_fields.into_without_relations().into_virtuals_last();

    let mut in_memory_ops =
        in_memory_processing::extract_in_memory_ops_for_nested_query(&mut rrq.args, has_unique_parent);

    let (mut child_query, join) = if rrq.parent_field.relation().is_many_to_many() {
        build_read_m2m_query(linkage, rrq.args, &selected_fields, builder)?
    } else {
        build_read_one2m_query(linkage, rrq.args, &selected_fields, builder)?
    };

    in_memory_ops.linking_fields = Some(join.fields.clone());

    child_query = in_memory_ops.into_expression(child_query);

    if !rrq.nested.is_empty() {
        child_query = add_inmemory_join(child_query, rrq.nested, builder)?;
    };

    Ok((child_query, join))
}

/// Returns the scalar fields that would be used to filter the children by. The returned fields
/// do not necessarily represent the actual SQL filter, since some of the underlying SQL fields
/// cannot be represented within our data model (for example m2m linking fields). This function
/// is primarily useful for inferring the correct types of parameters.
///
/// For one-to-one and one-to-many relations, this function returns the linking fields of the child
/// model. It is not correct to do the same for many-to-many relations though, because for them
/// the linking fields of the child do not link to the parent model's identifiers, but rather to
/// the linking table. Instead, we return the linking fields of the parent model, since that's
/// what would be used to query the linking table.
fn get_relation_scalars_for_filters(rf: &RelationField) -> Vec<ScalarField> {
    if rf.relation().is_many_to_many() {
        rf.left_scalars()
    } else {
        rf.related_field().left_scalars()
    }
}

fn get_single_relation_scalar_for_filters(rf: &RelationField) -> Option<ScalarField> {
    if rf.relation().is_many_to_many() {
        rf.single_left_scalar()
    } else {
        rf.related_field().single_left_scalar()
    }
}

fn build_read_m2m_query(
    linkage: RelationLinkage,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    let result = builder
        .build_get_related_records(linkage, args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    Ok((
        Expression::Query(result.query),
        JoinMetadata {
            fields: vec![result.linking_field_alias],
            is_relation_unique: false,
        },
    ))
}

fn build_read_one2m_query(
    linkage: RelationLinkage,
    mut args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    let (field, conditions_per_field) = linkage.into_parent_field_and_conditions();

    let mut filters = args
        .filter
        .take()
        .into_iter()
        .chain(conditions_per_field.flat_map(|(field, conditions)| {
            conditions.into_iter().map(move |condition| {
                Filter::Scalar(ScalarFilter {
                    condition,
                    projection: ScalarProjection::Single(field.clone()),
                    mode: QueryMode::Default,
                })
            })
        }));

    let filter = match (filters.next(), filters.next()) {
        (None, _) => Filter::And(Vec::new()),
        (Some(filter), None) => filter,
        (Some(first), Some(second)) => {
            let mut all_filters = Vec::with_capacity(2 + filters.size_hint().0);
            all_filters.push(first);
            all_filters.push(second);
            all_filters.extend(filters);
            Filter::And(all_filters)
        }
    };

    args.filter = Some(filter);

    let expr = build_get_records(
        builder,
        &field.related_model(),
        args,
        selected_fields,
        RelationLoadStrategy::Query,
    )?;

    Ok((
        expr,
        JoinMetadata {
            fields: field
                .related_field()
                .left_scalars()
                .iter()
                .map(|sf| sf.db_name().into())
                .collect(),
            is_relation_unique: !field.arity().is_list(),
        },
    ))
}

fn build_get_records(
    builder: &dyn QueryBuilder,
    model: &Model,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    relation_load_strategy: RelationLoadStrategy,
) -> Result<Expression, TranslateError> {
    Ok(builder
        .build_get_records(model, args, selected_fields, relation_load_strategy)
        .map_err(TranslateError::QueryBuildFailure)?
        .into_iter()
        .map(Expression::Query)
        .reduce(|acc, q| match acc {
            Expression::Concat(mut vec) => {
                vec.push(q);
                Expression::Concat(vec)
            }
            _ => Expression::Concat(vec![acc, q]),
        })
        .expect("should always have at least one query"))
}

fn convert_options_to_validation(expr: Expression, options: QueryOptions) -> Expression {
    if options.contains(QueryOption::ThrowOnEmpty) {
        let expectation =
            DataExpectation::non_empty_rows(MissingRecord::builder().operation(DataOperation::Query).build());
        Expression::validate_expectation(&expectation, expr)
    } else {
        expr
    }
}

#[derive(Debug, Default, Clone)]
struct JoinMetadata {
    fields: Vec<String>,
    is_relation_unique: bool,
}

impl JoinMetadata {
    fn into_fields(self) -> Vec<String> {
        self.fields
    }
}
