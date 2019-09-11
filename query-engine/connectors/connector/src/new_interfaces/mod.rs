use prisma_models::*;

pub trait Connector {
    fn with_transaction<F, T>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn MaybeTransaction) -> crate::Result<T>;

    //    fn with_connection<F, T>(&self, f: F) -> crate::Result<T>
    //    where
    //        F: FnOnce(&mut dyn MaybeTransaction) -> crate::Result<T>;
}

pub trait MaybeTransaction: ReadOperations + WriteOperations {}

pub trait ReadOperations {}
pub trait WriteOperations {
    fn connect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId) -> crate::Result<()>;

    fn disconnect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId)
        -> crate::Result<()>;

    fn set(&mut self, relation_field: RelationFieldRef, parent: GraphqlId, wheres: Vec<GraphqlId>)
        -> crate::Result<()>;
}
