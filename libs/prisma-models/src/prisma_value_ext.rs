use super::{GraphqlId, PrismaValue};
use crate::DomainError;

use std::convert::TryFrom;

pub trait PrismaValueExtensions {
    fn into_graphql_id(self) -> crate::Result<GraphqlId>;
}

impl PrismaValueExtensions for PrismaValue {
    fn into_graphql_id(self) -> Result<GraphqlId, DomainError> {
        let as_graphql_id = GraphqlId::try_from(self)?;
        Ok(as_graphql_id)
    }
}
