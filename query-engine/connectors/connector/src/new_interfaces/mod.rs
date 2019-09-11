use prisma_models::*;

pub trait Connector {
    fn with_transaction<F, T>(&self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn MaybeTransaction) -> crate::Result<T>;

    //    fn with_connection<F, T>(&self, f: F) -> crate::Result<T>
    //    where
    //        F: FnOnce(&mut dyn MaybeTransaction) -> crate::Result<T>;
}

pub struct WriteArgs {
    non_list_args: PrismaArgs,
    list_args: Vec<(String, PrismaListValue)>,
}
impl WriteArgs {
    pub fn new(non_list_args: PrismaArgs, list_args: Vec<(String, PrismaListValue)>) -> WriteArgs {
        WriteArgs {
            non_list_args,
            list_args,
        }
    }

    pub fn non_list_args(&self) -> &PrismaArgs {
        &self.non_list_args
    }

    pub fn list_args(&self) -> &Vec<(String, PrismaListValue)> {
        &self.list_args
    }
}

pub trait MaybeTransaction: ReadOperations + WriteOperations {}

pub trait ReadOperations {}
pub trait WriteOperations {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> crate::Result<GraphqlId>;

    fn connect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId) -> crate::Result<()>;

    fn disconnect(&mut self, field: RelationFieldRef, parent_id: &GraphqlId, child_id: &GraphqlId)
        -> crate::Result<()>;

    fn set(&mut self, relation_field: RelationFieldRef, parent: GraphqlId, wheres: Vec<GraphqlId>)
        -> crate::Result<()>;
}
