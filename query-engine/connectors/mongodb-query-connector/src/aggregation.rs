use crate::join::JoinStage;
use connector_interface::RelAggregationSelection;
use mongodb::bson::{doc, Document};

pub(crate) struct RelAggregationBuilder {
    aggregations: Vec<RelAggregationSelection>,
}

impl RelAggregationBuilder {
    pub fn new(aggregation_selections: &[RelAggregationSelection]) -> Self {
        Self {
            aggregations: aggregation_selections.to_vec(),
        }
    }

    pub(crate) fn build_joins(&self) -> Vec<JoinStage> {
        self.aggregations
            .iter()
            .map(|aggr| match aggr {
                RelAggregationSelection::Count(rf) => JoinStage {
                    source: rf.clone(),
                    alias: Some(aggr.db_alias()),
                    nested: vec![],
                },
            })
            .collect()
    }

    pub(crate) fn build_projections(&self) -> Vec<Document> {
        self.aggregations
            .iter()
            .map(|aggr| {
                let dollar_aggr_alias = format!("${}", aggr.db_alias());

                doc! {
                  aggr.db_alias(): { "$size": dollar_aggr_alias }
                }
            })
            .collect()
    }
}
