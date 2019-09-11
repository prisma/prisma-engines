use crate::query_builder::write::NestedActions;
use crate::query_builder::WriteQueryBuilder;
use crate::{
    query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlCapabilities, SqlError, Transaction, Transactional,
};
use connector_interface::*;
use datamodel::Source;
use prisma_models::*;
use prisma_query::{
    connector::{PostgresParams, Queryable},
    pool::{postgres::PostgresManager, PrismaConnectionManager},
};
use std::convert::TryFrom;

type Pool = r2d2::Pool<PrismaConnectionManager<PostgresManager>>;

pub struct PostgreSql {
    pool: Pool,
}

impl FromSource for PostgreSql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let url = url::Url::parse(&source.url().value)?;
        let params = PostgresParams::try_from(url)?;
        let pool = r2d2::Pool::try_from(params).unwrap();

        Ok(PostgreSql { pool })
    }
}

impl SqlCapabilities for PostgreSql {
    type ManyRelatedRecordsBuilder = ManyRelatedRecordsWithRowNumber;
}

impl Transactional for PostgreSql {
    fn with_transaction<F, T>(&self, _: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn Transaction) -> crate::Result<T>,
    {
        let mut conn = self.pool.get()?;
        let mut tx = conn.start_transaction()?;
        let result = f(&mut tx);

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }
}

impl Connector for PostgreSql {
    fn with_transaction<F, T>(&self, f: F) -> connector_interface::Result<T>
    where
        F: FnOnce(&mut dyn connector_interface::MaybeTransaction) -> connector_interface::Result<T>,
    {
        let mut conn = self.pool.get().map_err(SqlError::from)?;
        let tx = conn.start_transaction().map_err(SqlError::from)?;
        let mut connector_transaction = ConnectorTransaction { inner: tx };
        let result = f(&mut connector_transaction);

        if result.is_ok() {
            connector_transaction.inner.commit().map_err(SqlError::from)?;
        }

        result
    }
}

struct ConnectorTransaction<'a> {
    inner: prisma_query::connector::Transaction<'a>,
}
impl connector_interface::MaybeTransaction for ConnectorTransaction<'_> {}

impl connector_interface::ReadOperations for ConnectorTransaction<'_> {}
impl connector_interface::WriteOperations for ConnectorTransaction<'_> {
    fn connect(
        &mut self,
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector_interface::Result<()> {
        let query = WriteQueryBuilder::create_relation(field, parent_id, child_id);
        self.inner.execute(query).unwrap();
        Ok(())
    }

    fn disconnect(
        &mut self,
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector_interface::Result<()> {
        let child_model = field.related_model();
        let nested_disconnect = NestedDisconnect {
            relation_field: field,
            where_: Some(RecordFinder::new(child_model.fields().id(), child_id)),
        };
        let query = nested_disconnect.removal_by_parent_and_child(parent_id, child_id);
        self.inner.execute(query).unwrap();
        Ok(())
    }

    fn set(
        &mut self,
        _relation_field: RelationFieldRef,
        _parent: GraphqlId,
        _wheres: Vec<GraphqlId>,
    ) -> connector_interface::Result<()> {
        unimplemented!()
    }
}
