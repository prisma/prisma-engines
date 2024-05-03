use crate::join::JoinStage;
use itertools::Itertools;
use mongodb::bson::{doc, Document};
use query_structure::{OrderBy, OrderByHop, OrderByToManyAggregation, SortOrder};
use std::{fmt::Display, iter};

#[derive(Debug)]
pub(crate) struct OrderByData {
    /// All collection join stages required to make this orderBy's data available for sorting.
    pub(crate) join: Option<JoinStage>,

    /// The path prefix of the scalar or aggregate we're ordering by.
    /// Can be multiple hops over relations and composites.
    pub(crate) prefix: Option<OrderByPrefix>,

    /// The base order object from the query core this data is based on.
    pub(crate) order_by: OrderBy,
}

impl OrderByData {
    pub(crate) fn from_list(order_bys: Vec<OrderBy>) -> Vec<Self> {
        order_bys
            .into_iter()
            .enumerate()
            .map(|(index, ordering)| OrderByData::compute(ordering, index))
            .collect()
    }

    pub(crate) fn compute(order_by: OrderBy, index: usize) -> Self {
        let prefix = Self::compute_prefix(index, &order_by);

        // Determine if we need to compute join stages.
        if !order_by.contains_relation_hops() {
            Self {
                join: None,
                prefix,
                order_by,
            }
        } else {
            let prefix = prefix.unwrap(); // Safe because relation hops always produce a prefix.
            let stages = order_by
                .path()
                .unwrap() // Safe because relation hops always have a path.
                .iter()
                .filter_map(|hop| match hop {
                    OrderByHop::Relation(rf) => Some(JoinStage::new(rf.clone())),
                    OrderByHop::Composite(_) => None, // We don't need to join if we're looking at composites - they're already present on the document.
                })
                .collect_vec();

            // We fold from right to left because the right hand side needs to be always contained
            // in the left hand side here (JoinStage<A, JoinStage<B, JoinStage<C>>>).
            let mut folded_stages = stages
                .into_iter()
                .rev()
                .reduce(|right, mut left| {
                    left.push_nested(right);
                    left
                })
                .unwrap();

            folded_stages.set_alias(prefix.first().unwrap().to_string());

            Self {
                join: Some(folded_stages),
                prefix: Some(prefix),
                order_by,
            }
        }
    }

    fn compute_prefix(index: usize, order_by: &OrderBy) -> Option<OrderByPrefix> {
        match order_by.path() {
            Some(path) => {
                let mut parts = path
                    .iter()
                    .map(|hop| match hop {
                        OrderByHop::Relation(rf) => rf.relation().name(),
                        OrderByHop::Composite(cf) => cf.db_name().to_owned(),
                    })
                    .collect_vec();

                if parts.is_empty() {
                    None
                } else {
                    let clone_to = if order_by.contains_relation_hops() {
                        let alias = format!("orderby_{}_{}", parts[0], index);
                        parts[0] = alias;

                        None
                    } else if !path.is_empty() {
                        // Case for composite-only path.
                        Some(format!("orderby_{}_{}", parts[0], index))
                    } else {
                        None
                    };

                    Some(OrderByPrefix::new(parts, clone_to))
                }
            }
            None => None,
        }
    }

    /// The Mongo binding name of this orderBy, required for cursor conditions.
    /// For a relation field, this is only the first item of the path (e.g. orderby_TestModel_1)
    /// Returns 2 forms, one for the left one and one for the right one. The left one is a unique,
    /// valid name for a variable binding.
    pub(crate) fn binding_names(&self) -> (String, String) {
        if let Some(ref prefix) = self.prefix {
            let first = prefix.first().unwrap().to_string();

            (first.clone(), first)
        } else {
            // TODO: Order by relevance won't work here
            let field = self.order_by.field().expect("a field on which to order by is expected");
            let right = field.db_name().to_owned();
            let mut left = field.name().to_owned();

            // Issue: https://github.com/prisma/prisma/issues/14001
            // Here we can assume the field name is ASCII, because it is the _client_ field name,
            // not the mapped name.
            left[..1].make_ascii_lowercase();

            (left, right)
        }
    }

    /// The name of the scalar the ordering (ultimately) refers to.
    pub(crate) fn scalar_field_name(&self) -> String {
        // TODO: Order by relevance won't work here
        let field = self.order_by.field().expect("a field on which to order by is expected");

        field.db_name().to_string()
    }

    /// Computes the full query path that would be required to traverse from the top-most
    /// document all the way to the scalar to order through all hops.
    pub(crate) fn full_reference_path(&self, use_bindings: bool) -> String {
        if matches!(
            self.order_by,
            OrderBy::ScalarAggregation(_) | OrderBy::ToManyAggregation(_)
        ) {
            // Order by aggregates are always referenced by their join prefix and not a specific field name.
            self.prefix.as_ref().unwrap().to_string()
        } else if let Some(ref prefix) = self.prefix {
            iter::once(prefix.to_string())
                .chain(iter::once(self.scalar_field_name()))
                .join(".")
        } else if use_bindings {
            self.binding_names().0
        } else {
            self.prefix
                .as_ref()
                .into_iter()
                .map(ToString::to_string)
                .chain(iter::once(self.scalar_field_name()))
                .join(".")
        }
    }

    pub(crate) fn sort_order(&self) -> SortOrder {
        self.order_by.sort_order()
    }
}

#[derive(Debug)]
pub(crate) struct OrderByPrefix {
    /// Instructs the query to clone the target (e.g. a scalar field) to the given clone alias instead of
    /// folding the field piece by piece as the joins do. Only used for composite-only cases,
    /// e.g. count number of `model.to_one_com.to_many_com`, where we would override the composite on the document,
    /// leaving us without means to also query it for other purposes (like orderBy count + read the composite).
    clone_to: Option<String>,

    /// Parts of the path, stringified.
    parts: Vec<String>,
}

impl Display for OrderByPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.parts.join("."))
    }
}

impl OrderByPrefix {
    pub(crate) fn new(parts: Vec<String>, clone_to: Option<String>) -> Self {
        Self { parts, clone_to }
    }

    pub(crate) fn first(&self) -> Option<&String> {
        self.parts.first()
    }
}

/// Builder for `sort` mongo documents.
/// Building of orderBy needs to be deferred until all other args are complete
/// to have all information necessary to build the correct sort arguments.
#[derive(Debug, Default)]
pub(crate) struct OrderByBuilder {
    order_bys: Vec<OrderByData>,
    reverse: bool,
}

impl OrderByBuilder {
    pub fn new(order_bys: Vec<OrderBy>, reverse: bool) -> Self {
        let order_bys = order_bys
            .into_iter()
            .enumerate()
            .map(|(index, ordering)| OrderByData::compute(ordering, index))
            .collect();

        Self { order_bys, reverse }
    }

    /// Builds and renders a Mongo sort document.
    /// `is_group_by` signals that the ordering is for a grouping,
    /// requiring a prefix to refer to the correct document nesting.
    pub(crate) fn build(self, is_group_by: bool) -> (Option<Document>, Vec<Document>, Vec<JoinStage>) {
        if self.order_bys.is_empty() {
            return (None, vec![], vec![]);
        }

        let mut order_doc = Document::new();
        let mut order_aggregate_proj_doc: Vec<Document> = vec![];
        let mut joins = vec![];

        for data in self.order_bys.into_iter() {
            let is_to_many_aggregation = matches!(&data.order_by, OrderBy::ToManyAggregation(_));
            let mut field_name = if is_group_by {
                // Can only be scalar aggregations for groupBy, ToMany aggregations are not supported yet.
                if let OrderBy::ScalarAggregation(order_by_aggr) = &data.order_by {
                    let prefix = match order_by_aggr.sort_aggregation {
                        query_structure::SortAggregation::Count => "count",
                        query_structure::SortAggregation::Avg => "avg",
                        query_structure::SortAggregation::Sum => "sum",
                        query_structure::SortAggregation::Min => "min",
                        query_structure::SortAggregation::Max => "max",
                    };

                    format!("{}_{}", prefix, data.scalar_field_name())
                } else {
                    // Explanation: All group by fields go into the _id key of the result document.
                    // As it is the only point where the flat scalars are contained for the group,
                    // we need to refer to the object.
                    format!("_id.{}", data.scalar_field_name())
                }
            } else if is_to_many_aggregation {
                // Since Order by to-many aggregate with middle to-one path will be unwinded,
                // we need to refer to it with its top-level join alias
                data.binding_names().0
            } else {
                data.full_reference_path(false)
            };

            // Unwind order by aggregate to-one middle joins into the top level join
            // to prevent nested join result which break the stages that come after
            // See `unwind_aggregate_joins` for more explanation
            if let OrderBy::ToManyAggregation(order_by_aggregate) = &data.order_by {
                if !order_by_aggregate.path.is_empty() {
                    match order_by_aggregate.sort_aggregation {
                        query_structure::SortAggregation::Count => {
                            if let Some(clone_to) = data.prefix.as_ref().and_then(|x| x.clone_to.clone()) {
                                order_aggregate_proj_doc.push(doc! { "$addFields": { clone_to.clone(): { "$size": { "$ifNull": [format!("${}", data.full_reference_path(false)), []] } } } });
                                field_name = clone_to; // Todo: Just a hack right now, this whole function needs love.
                            } else {
                                order_aggregate_proj_doc.extend(unwind_aggregate_joins(
                                    field_name.clone().as_str(),
                                    order_by_aggregate,
                                    &data,
                                ));

                                order_aggregate_proj_doc.push(doc! { "$addFields": { field_name.clone(): { "$size": { "$ifNull": [format!("${field_name}"), []] } } } });
                            }
                        }
                        _ => unimplemented!("Order by aggregate only supports COUNT"),
                    }
                }
            }

            // Mongo: -1 -> DESC, 1 -> ASC
            match (data.sort_order(), self.reverse) {
                (SortOrder::Ascending, true) => order_doc.insert(field_name, -1),
                (SortOrder::Descending, true) => order_doc.insert(field_name, 1),
                (SortOrder::Ascending, false) => order_doc.insert(field_name, 1),
                (SortOrder::Descending, false) => order_doc.insert(field_name, -1),
            };

            joins.extend(data.join);
        }

        (Some(order_doc), order_aggregate_proj_doc, joins)
    }
}

/// In order to enable computing aggregation on nested joins, we unwind & replace the top-level
/// join by the nested to-one joins so that we can apply a $size operation.
///
/// Note: In theory, we do not need to unwind composites, they are already present as single object (to-one) or array (to-many), but we do it anyways here.
/// Note: This is not necessary for `ScalarAggregations`. Todo [Dom] Why? It's not entirely clear...
///
/// Let's consider these relations:
/// (one or many) A to-one B to-one C to-many D
/// We'll get the following joins result: { orderby_AToB: [{ BToC: [{ CToD: [1, 2, 3] }] }] }
/// This function will generate the following stages:
/// 1. { $unwind: { path: "$orderby_AToB" } } -> { orderby_AToB: { BToC: [{ CToD: [1, 2, 3] }] } }
/// 2. { $addFields: { orderby_AToB: "$orderby_AToB.BToC" } } -> [{ CToD: [1, 2, 3] }]
/// 3. { $unwind: { path: "$orderby_AToB" } } ->  { CToD: [1, 2, 3] }
/// 4. { $addFields: { orderby_AToB: "$orderby_AToB.CToD" } } -> [1, 2, 3]
fn unwind_aggregate_joins(
    join_name: &str,
    order_by_aggregate: &OrderByToManyAggregation,
    data: &OrderByData,
) -> Vec<Document> {
    order_by_aggregate
        .path
        .iter()
        .enumerate()
        .filter_map(|(i, hop)| match hop {
            OrderByHop::Composite(cf) if cf.is_list() => None,
            OrderByHop::Relation(rf) if rf.is_list() => None,
            OrderByHop::Relation(_) | OrderByHop::Composite(_) => {
                let mut additional_stages = vec![];

                if let Some(next_part) = data.prefix.as_ref().and_then(|prefix| prefix.parts.get(i + 1)) {
                    additional_stages.push(doc! {
                        "$unwind": {
                            "path": format!("${join_name}"),
                            "preserveNullAndEmptyArrays": true
                        }
                    });

                    additional_stages.push(doc! { "$addFields": { join_name: format!("${join_name}.{next_part}") } });
                }

                Some(additional_stages)
            }
        })
        .flatten()
        .collect_vec()
}
