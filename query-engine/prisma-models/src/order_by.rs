use crate::{CompositeFieldRef, RelationFieldRef, ScalarFieldRef};
use quaint::prelude::Order;
use std::string::ToString;

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn into_order(self, reverse: bool) -> Order {
        match (self, reverse) {
            (SortOrder::Ascending, false) => Order::Asc,
            (SortOrder::Descending, false) => Order::Desc,
            (SortOrder::Ascending, true) => Order::Desc,
            (SortOrder::Descending, true) => Order::Asc,
        }
    }
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
    Aggregation(OrderByAggregation),
    Relevance(OrderByRelevance),
}

impl OrderBy {
    pub fn path(&self) -> Vec<OrderByHop> {
        match self {
            OrderBy::Scalar(o) => o.path.clone(),
            OrderBy::Aggregation(o) => o.path.clone(),
            OrderBy::Relevance(_) => vec![],
        }
    }

    pub fn sort_order(&self) -> SortOrder {
        match self {
            OrderBy::Scalar(o) => o.sort_order,
            OrderBy::Aggregation(o) => o.sort_order,
            OrderBy::Relevance(o) => o.sort_order,
        }
    }

    pub fn field(&self) -> Option<ScalarFieldRef> {
        match self {
            OrderBy::Scalar(o) => Some(o.field.clone()),
            OrderBy::Aggregation(o) => o.field.clone(),
            OrderBy::Relevance(_) => None,
        }
    }

    pub fn has_middle_to_one_path(&self) -> bool {
        let path = self.path();
        let len = path.len();

        if len < 2 {
            false
        } else {
            path.get(len - 2)
                .map(|hop| match hop {
                    OrderByHop::Relation(rf) => !rf.is_list(),
                    OrderByHop::Composite(cf) => !cf.is_list(),
                })
                .unwrap_or(false)
        }
    }

    pub fn scalar(field: ScalarFieldRef, path: Vec<OrderByHop>, sort_order: SortOrder) -> Self {
        Self::Scalar(OrderByScalar {
            field,
            path,
            sort_order,
        })
    }

    pub fn aggregation(
        field: Option<ScalarFieldRef>,
        path: Vec<OrderByHop>,
        sort_order: SortOrder,
        sort_aggregation: SortAggregation,
    ) -> Self {
        Self::Aggregation(OrderByAggregation {
            field,
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
pub struct OrderByAggregation {
    pub field: Option<ScalarFieldRef>,
    pub path: Vec<OrderByHop>,
    pub sort_order: SortOrder,
    pub sort_aggregation: SortAggregation,
}

impl OrderByAggregation {
    // pub fn field(&self) -> ScalarFieldRef {
    //     match &self.field {
    //         Some(sf) => sf.clone(),
    //         // TODO: This is a hack that should be removed once MongoDB is refactored too
    //         None => self.id_field_from_relation(),
    //     }
    // }

    // fn id_field_from_relation(&self) -> ScalarFieldRef {
    //     let ids: Vec<_> = self
    //         .path
    //         .last()
    //         .unwrap()
    //         .related_model()
    //         .primary_identifier()
    //         .as_scalar_fields()
    //         .expect("Primary identifier contains non-scalar fields.");

    //     ids.into_iter().next().unwrap()
    // }

    pub fn is_scalar_aggregation(&self) -> bool {
        self.field.is_some() && self.path.is_empty()
    }
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
