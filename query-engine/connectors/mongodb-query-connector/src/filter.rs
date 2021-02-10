use std::unimplemented;

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

            Filter::Scalar(sf) => sf.into_bson(),
            Filter::Empty => Ok(Document::new().into()),
            // Filter::ScalarList(slf) => {}
            // Filter::OneRelationIsNull(_) => {}
            // Filter::Relation(_) => {}
            // Filter::BoolFilter(b) => {} // Potentially not doable.
            // Filter::Aggregation(_) => {}
            _ => todo!("Incomplete filter implementation."),
        }
    }
}

// Todo:
// - case insensitive (probably regex or text search?)
impl IntoBson for ScalarFilter {
    fn into_bson(self) -> crate::Result<Bson> {
        // Todo: Find out what Compound cases are
        let field = match self.projection {
            connector_interface::ScalarProjection::Single(sf) => sf,
            connector_interface::ScalarProjection::Compound(_) => unimplemented!("Compound filter case."),
        };

        // let mode = self.mode;

        let filter = match self.condition {
            ScalarCondition::Equals(val) => doc! { "eq": val.into_bson()? },
            ScalarCondition::NotEquals(val) => doc! { "ne": val.into_bson()? },
            ScalarCondition::Contains(_val) => todo!(),
            ScalarCondition::NotContains(_val) => todo!(),
            ScalarCondition::StartsWith(_val) => todo!(),
            ScalarCondition::NotStartsWith(_val) => todo!(),
            ScalarCondition::EndsWith(_val) => todo!(),
            ScalarCondition::NotEndsWith(_val) => todo!(),
            ScalarCondition::LessThan(val) => doc! { "lt": val.into_bson()? },
            ScalarCondition::LessThanOrEquals(val) => doc! { "lte": val.into_bson()? },
            ScalarCondition::GreaterThan(val) => doc! { "gt": val.into_bson()? },
            ScalarCondition::GreaterThanOrEquals(val) => doc! { "gte": val.into_bson()? },
            ScalarCondition::In(vals) => doc! { "in": PrismaValue::List(vals).into_bson()? },
            ScalarCondition::NotIn(vals) => doc! { "nin": PrismaValue::List(vals).into_bson()? },
        };

        Ok(doc! { field.db_name(): filter }.into())
    }
}
