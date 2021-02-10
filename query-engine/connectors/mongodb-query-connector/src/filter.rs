use connector_interface::{Filter, ScalarCondition, ScalarFilter};
use mongodb::bson::{doc, Bson, Document};
use prisma_models::PrismaValue;

use crate::IntoBson;

// Mongo filters are a BSON document.
impl IntoBson for Filter {
    fn into_bson(self) -> crate::Result<Bson> {
        match self {
            Filter::And(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$and": Bson::Array(filters) }.into())
            }

            Filter::Or(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$or": Bson::Array(filters) }.into())
            }

            Filter::Not(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$not": Bson::Array(filters) }.into())
            }

            // Filter::Scalar(sf) => {}
            // Filter::ScalarList(slf) => {}
            // Filter::OneRelationIsNull(_) => {}
            // Filter::Relation(_) => {}
            // Filter::BoolFilter(_) => {}
            // Filter::Aggregation(_) => {}
            // Filter::Empty => {}
            _ => todo!(),
        }
    }
}

// Todo:
// - case insensitive
impl IntoBson for ScalarFilter {
    fn into_bson(self) -> crate::Result<Bson> {
        let cond = match &self.condition {
            ScalarCondition::Equals(PrismaValue::Null) => doc! { "eq": Bson::Null },
            ScalarCondition::NotEquals(PrismaValue::Null) => doc! { "ne": Bson::Null },
            ScalarCondition::Equals(val) => doc! { "eq": Bson::Null },
            ScalarCondition::NotEquals(_) => doc! { "ne": Bson::Null },
            ScalarCondition::Contains(_) => doc! { "ne": Bson::Null },
            ScalarCondition::NotContains(_) => doc! { "ne": Bson::Null },
            ScalarCondition::StartsWith(_) => doc! { "ne": Bson::Null },
            ScalarCondition::NotStartsWith(_) => doc! { "ne": Bson::Null },
            ScalarCondition::EndsWith(_) => doc! { "ne": Bson::Null },
            ScalarCondition::NotEndsWith(_) => doc! { "ne": Bson::Null },
            ScalarCondition::LessThan(_) => doc! { "ne": Bson::Null },
            ScalarCondition::LessThanOrEquals(_) => doc! { "ne": Bson::Null },
            ScalarCondition::GreaterThan(_) => doc! { "ne": Bson::Null },
            ScalarCondition::GreaterThanOrEquals(_) => doc! { "ne": Bson::Null },
            ScalarCondition::In(_) => doc! { "ne": Bson::Null },
            ScalarCondition::NotIn(_) => doc! { "ne": Bson::Null },
        };

        Ok(cond.into())
    }
}
