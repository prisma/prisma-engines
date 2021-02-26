use crate::join::JoinStage;
use itertools::Itertools;
use mongodb::bson::Document;
use prisma_models::{OrderBy, SortOrder};

/// Builder for `sort` mongo documents.
/// Building of orderBy needs to be deferred until all other args are complete
/// to have all information necessary to build the correct sort arguments.
#[derive(Debug, Default)]
pub(crate) struct OrderByBuilder {
    order_bys: Vec<OrderBy>,
    reverse: bool,
}

impl OrderByBuilder {
    pub fn new(order_bys: Vec<OrderBy>, reverse: bool) -> Self {
        Self { order_bys, reverse }
    }

    /// Builds and renders a Mongo sort document.
    /// `is_group_by` signals that the ordering is for a grouping,
    /// requiring a prefix to refer to the correct document nesting.
    pub(crate) fn build(self, is_group_by: bool) -> (Option<Document>, Vec<JoinStage>) {
        if self.order_bys.is_empty() {
            return (None, vec![]);
        }

        let mut order_doc = Document::new();
        let mut joins = vec![];

        for (index, order_by) in self.order_bys.into_iter().enumerate() {
            let prefix = if !order_by.path.is_empty() {
                let mut prefix = order_by.path.iter().map(|rf| rf.relation().name.clone()).collect_vec();
                let mut stages = order_by.path.into_iter().map(|rf| JoinStage::new(rf)).collect_vec();

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

                let alias = format!("orderby_{}_{}", prefix[0], index);

                final_stage.set_alias(alias.clone());
                joins.push(final_stage);
                prefix[0] = alias;

                Some(prefix.join("."))
            } else {
                None
            };

            let field = if is_group_by {
                // Explanation: All group by fields go into the _id key of the result document.
                // As it is the only point where the flat scalars are contained for the group,
                // we beed to refer to the object
                format!("_id.{}", order_by.field.db_name())
            } else {
                if let Some(prefix) = prefix {
                    format!("{}.{}", prefix, order_by.field.db_name())
                } else {
                    order_by.field.db_name().to_owned()
                }
            };

            // Mongo: -1 -> DESC, 1 -> ASC
            match (order_by.sort_order, self.reverse) {
                (SortOrder::Ascending, true) => order_doc.insert(field, -1),
                (SortOrder::Descending, true) => order_doc.insert(field, 1),
                (SortOrder::Ascending, false) => order_doc.insert(field, 1),
                (SortOrder::Descending, false) => order_doc.insert(field, -1),
            };
        }

        (Some(order_doc), joins)
    }
}
