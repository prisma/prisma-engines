use crate::{CompositeFieldRef, RelationFieldRef, ScalarFieldRef};
use std::string::ToString;

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortAggregation {
    Count,
    Avg,
    Sum,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderBy {
    Scalar(OrderByScalar),
    ScalarAggregation(OrderByScalarAggregation),
    ToManyAggregation(OrderByToManyAggregation),
    Relevance(OrderByRelevance),
}

impl OrderBy {
    pub fn path(&self) -> Option<&[OrderByHop]> {
        match self {
            OrderBy::Scalar(o) => Some(&o.path),
            OrderBy::ScalarAggregation(o) => Some(&o.path),
            OrderBy::ToManyAggregation(o) => Some(&o.path),
            OrderBy::Relevance(_) => None,
        }
    }

    pub fn sort_order(&self) -> SortOrder {
        match self {
            OrderBy::Scalar(o) => o.sort_order,
            OrderBy::ScalarAggregation(o) => o.sort_order,
            OrderBy::ToManyAggregation(o) => o.sort_order,
            OrderBy::Relevance(o) => o.sort_order,
        }
    }

    pub fn field(&self) -> Option<ScalarFieldRef> {
        match self {
            OrderBy::Scalar(o) => Some(o.field.clone()),
            OrderBy::ScalarAggregation(o) => Some(o.field.clone()),
            OrderBy::ToManyAggregation(_) => None,
            OrderBy::Relevance(_) => None,
        }
    }

    pub fn contains_relation_hops(&self) -> bool {
        match self.path() {
            Some(path) => path.iter().any(|hop| matches!(hop, &OrderByHop::Relation(_))),
            None => false,
        }
    }

    pub fn scalar(field: ScalarFieldRef, path: Vec<OrderByHop>, sort_order: SortOrder) -> Self {
        Self::Scalar(OrderByScalar {
            field,
            path,
            sort_order,
        })
    }

    pub fn scalar_aggregation(
        field: ScalarFieldRef,
        path: Vec<OrderByHop>,
        sort_order: SortOrder,
        sort_aggregation: SortAggregation,
    ) -> Self {
        Self::ScalarAggregation(OrderByScalarAggregation {
            field,
            path,
            sort_order,
            sort_aggregation,
        })
    }

    pub fn to_many_aggregation(
        path: Vec<OrderByHop>,
        sort_order: SortOrder,
        sort_aggregation: SortAggregation,
    ) -> Self {
        Self::ToManyAggregation(OrderByToManyAggregation {
            path,
            sort_order,
            sort_aggregation,
        })
    }

    pub fn relevance(fields: Vec<ScalarFieldRef>, search: String, sort_order: SortOrder) -> Self {
        Self::Relevance(OrderByRelevance {
            fields,
            sort_order,
            search,
        })
    }
}

/// Describes a hop over to a relation or composite for an orderBy statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderByHop {
    Relation(RelationFieldRef),
    Composite(CompositeFieldRef),
}

impl OrderByHop {
    pub fn into_relation_hop(&self) -> Option<&RelationFieldRef> {
        match self {
            OrderByHop::Relation(rf) => Some(rf),
            OrderByHop::Composite(_) => None,
        }
    }
}

impl From<&RelationFieldRef> for OrderByHop {
    fn from(rf: &RelationFieldRef) -> Self {
        rf.clone().into()
    }
}

impl From<&CompositeFieldRef> for OrderByHop {
    fn from(cf: &CompositeFieldRef) -> Self {
        cf.clone().into()
    }
}

impl From<RelationFieldRef> for OrderByHop {
    fn from(rf: RelationFieldRef) -> Self {
        Self::Relation(rf)
    }
}

impl From<CompositeFieldRef> for OrderByHop {
    fn from(cf: CompositeFieldRef) -> Self {
        Self::Composite(cf)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderByScalar {
    pub field: ScalarFieldRef,
    pub path: Vec<OrderByHop>,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderByScalarAggregation {
    pub field: ScalarFieldRef,
    pub path: Vec<OrderByHop>,
    pub sort_order: SortOrder,
    pub sort_aggregation: SortAggregation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderByToManyAggregation {
    pub path: Vec<OrderByHop>,
    pub sort_order: SortOrder,
    pub sort_aggregation: SortAggregation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderByRelevance {
    pub fields: Vec<ScalarFieldRef>,
    pub sort_order: SortOrder,
    pub search: String,
}

impl ToString for SortOrder {
    fn to_string(&self) -> String {
        match self {
            SortOrder::Ascending => String::from("ASC"),
            SortOrder::Descending => String::from("DESC"),
        }
    }
}

impl From<ScalarFieldRef> for OrderBy {
    fn from(field: ScalarFieldRef) -> Self {
        Self::Scalar(OrderByScalar {
            field,
            path: vec![],
            sort_order: SortOrder::Ascending,
        })
    }
}
