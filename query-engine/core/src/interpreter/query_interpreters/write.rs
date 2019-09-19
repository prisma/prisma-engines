use crate::{interpreter::InterpretationResult, query_ast::*, QueryResult};
use connector::TransactionLike;

pub fn execute(tx: &mut dyn TransactionLike, write_query: WriteQuery) -> InterpretationResult<QueryResult> {
    match write_query {
        WriteQuery::CreateRecord(q) => create_one(tx, q),
        WriteQuery::UpdateRecord(q) => unimplemented!(),
        WriteQuery::DeleteRecord(q) => unimplemented!(),
        WriteQuery::UpdateManyRecords(q) => unimplemented!(),
        WriteQuery::DeleteManyRecords(q) => unimplemented!(),
        WriteQuery::ConnectRecords(q) => unimplemented!(),
        WriteQuery::DisconnectRecords(q) => unimplemented!(),
        WriteQuery::SetRecords(q) => unimplemented!(),
        WriteQuery::ResetData(q) => unimplemented!(),
    }
}

fn create_one(tx: &mut dyn TransactionLike, q: CreateRecord) -> InterpretationResult<QueryResult> {
    unimplemented!()
}
