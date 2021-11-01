use crate::join::JoinStage;
use itertools::Itertools;
use mongodb::bson::{doc, Document};
use prisma_models::{OrderBy, SortOrder};

#[derive(Debug)]
pub(crate) struct OrderByData {
    pub(crate) join: Option<JoinStage>,
    pub(crate) prefix: Option<OrderByPrefix>,
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
        if order_by.path().is_empty() {
            Self {
                join: None,
                prefix: None,
                order_by,
            }
        } else {
            let prefix = Self::compute_prefix(index, &order_by);
            let mut stages = order_by
                .path()
                .iter()
                .map(|rf| JoinStage::new(rf.clone()))
                .collect_vec();

            // We fold from right to left because the right hand side needs to be always contained
            // in the left hand side here (JoinStage<A, JoinStage<B, JoinStage<C>>>).
            stages.reverse();

            let mut final_stage = stages
                .into_iter()
                .fold1(|right, mut left| {
                    left.push_nested(right);
                    left
                })
                .unwrap();

            final_stage.set_alias(prefix.first().unwrap().to_string());

            Self {
                join: Some(final_stage),
                prefix: Some(prefix),
                order_by,
            }
        }
    }

    fn compute_prefix(index: usize, order_by: &OrderBy) -> OrderByPrefix {
        let mut parts = order_by
            .path()
            .iter()
            .map(|rf| rf.relation().name.clone())
            .collect_vec();
        let alias = format!("orderby_{}_{}", parts[0], index);

        parts[0] = alias;

        OrderByPrefix::new(parts)
    }

    /// The Mongo binding name of this orderBy, required for cursor conditions.
    /// For a relation field, this is only the first item of the path (e.g. orderby_TestModel_1)
    /// Returns 2 forms, one for the left one and one for the right one. The left one is
    /// escaped for usage in user-defined variables.
    pub(crate) fn binding_names(&self) -> (String, String) {
        if let Some(ref prefix) = self.prefix {
            let first = prefix.first().unwrap().to_string();

            (first.clone(), first)
        } else {
            // TODO: Order by relevance won't work here
            let right = self
                .order_by
                .field()
                .expect("a field on which to order by is expected")
                .db_name()
                .to_owned();

            let left = if right.starts_with('_') {
                right.strip_prefix('_').unwrap().to_owned()
            } else {
                right.to_owned()
            };

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
        if let Some(ref prefix) = self.prefix {
            // Order by aggregates are always referenced by their join prefix and not a specific field name.
            // since they are performed on relations
            if matches!(self.order_by, OrderBy::Aggregation(_)) {
                prefix.to_string()
            } else {
                format!("{}.{}", prefix.to_string(), self.scalar_field_name())
            }
        } else if use_bindings {
            self.binding_names().0
        } else {
            self.scalar_field_name()
        }
    }

    pub(crate) fn sort_order(&self) -> SortOrder {
        self.order_by.sort_order()
    }
}

#[derive(Debug)]
pub(crate) struct OrderByPrefix {
    parts: Vec<String>,
}

impl ToString for OrderByPrefix {
    fn to_string(&self) -> String {
        self.parts.join(".")
    }
}

impl OrderByPrefix {
    pub(crate) fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }

    pub(crate) fn first(&self) -> Option<&String> {
        self.parts.get(0)
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
            let field = if is_group_by {
                if let OrderBy::Aggregation(order_by_aggr) = &data.order_by {
                    let prefix = match order_by_aggr.sort_aggregation {
                        prisma_models::SortAggregation::Count => "count",
                        prisma_models::SortAggregation::Avg => "avg",
                        prisma_models::SortAggregation::Sum => "sum",
                        prisma_models::SortAggregation::Min => "min",
                        prisma_models::SortAggregation::Max => "max",
                    };

                    format!("{}_{}", prefix, data.scalar_field_name())
                } else {
                    // Explanation: All group by fields go into the _id key of the result document.
                    // As it is the only point where the flat scalars are contained for the group,
                    // we need to refer to the object
                    format!("_id.{}", data.scalar_field_name())
                }
            // Since Order by aggregate with middle to-one path will be unwinded,
            // we need to refer to it with its top-level join alias
            } else if matches!(&data.order_by, OrderBy::Aggregation(_)) && data.order_by.has_middle_to_one_path() {
                data.binding_names().0
            } else {
                data.full_reference_path(false)
            };

            // Unwind order by aggregate to-one middle joins into the top level join
            // to prevent nested join result which break the stages that come after
            // See `unwind_aggregate_joins` for more explanation
            if let OrderBy::Aggregation(order_by_aggregate) = &data.order_by {
                if !order_by_aggregate.path.is_empty() {
                    match order_by_aggregate.sort_aggregation {
                        prisma_models::SortAggregation::Count => {
                            if data.order_by.has_middle_to_one_path() {
                                order_aggregate_proj_doc.extend(unwind_aggregate_joins(
                                    field.clone().as_str(),
                                    order_by_aggregate,
                                    &data,
                                ));
                            }

                            order_aggregate_proj_doc.push(
                                doc! { "$addFields": { field.clone(): { "$size": format!("${}", field.clone()) } } },
                            );
                        }
                        _ => unimplemented!("Order by aggregate only supports COUNT"),
                    }
                }
            }

            // Mongo: -1 -> DESC, 1 -> ASC
            match (data.sort_order(), self.reverse) {
                (SortOrder::Ascending, true) => order_doc.insert(field, -1),
                (SortOrder::Descending, true) => order_doc.insert(field, 1),
                (SortOrder::Ascending, false) => order_doc.insert(field, 1),
                (SortOrder::Descending, false) => order_doc.insert(field, -1),
            };

            joins.extend(data.join);
        }

        (Some(order_doc), order_aggregate_proj_doc, joins)
    }
}

/// In order to enable computing aggregation on nested joins,
/// we unwind & replace the top-level join by the nested to-one joins so that we can apply a $size operation.
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
    order_by_aggregate: &prisma_models::OrderByAggregation,
    data: &OrderByData,
) -> Vec<Document> {
    order_by_aggregate
        .path
        .iter()
        .enumerate()
        .filter_map(|(i, rf)| {
            if rf.is_list {
                None
            } else {
                // Prefix parts are mapped 1-1 with order by path.
                // We can safely access (i + 1) here since the last path cannot be a to-one relation for an order by aggregate.
                let next_part_name = data.prefix.as_ref().unwrap().parts.get(i + 1).unwrap();

                Some(vec![
                    doc! {
                        "$unwind": {
                            "path": format!("${}", join_name),
                            "preserveNullAndEmptyArrays": true
                        }
                    },
                    doc! { "$addFields": { join_name: format!("${}.{}", join_name, next_part_name) } },
                ])
            }
        })
        .flatten()
        .collect_vec()
}
