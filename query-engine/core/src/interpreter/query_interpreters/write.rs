use crate::{
    interpreter::{InterpretationResult, InterpreterError},
    query_ast::*,
    QueryResult,
};
use connector::{Filter, Transaction, WriteArgs};

pub fn execute(tx: Box<dyn Transaction>, write_query: WriteQuery) -> InterpretationResult<QueryResult> {
    match write_query {
        WriteQuery::CreateRecord(q) => create_one(tx, q),
        WriteQuery::UpdateRecord(q) => update_one(tx, q),
        WriteQuery::DeleteRecord(q) => delete_one(tx, q),
        WriteQuery::UpdateManyRecords(q) => update_many(tx, q),
        WriteQuery::DeleteManyRecords(q) => delete_many(tx, q),
        WriteQuery::ConnectRecords(q) => connect(tx, q),
        WriteQuery::DisconnectRecords(q) => disconnect(tx, q),
        WriteQuery::SetRecords(q) => set(tx, q),
        WriteQuery::ResetData(q) => reset(tx, q),
    }
}

fn create_one(tx: Box<dyn Transaction>, q: CreateRecord) -> InterpretationResult<QueryResult> {
    let res = tx.create_record(q.model, WriteArgs::new(q.non_list_args, q.list_args))?;

    Ok(QueryResult::Id(res))
}

fn update_one(tx: Box<dyn Transaction>, q: UpdateRecord) -> InterpretationResult<QueryResult> {
    let mut res = tx.update_records(
        q.model,
        Filter::from(q.where_),
        WriteArgs::new(q.non_list_args, q.list_args),
    )?;

    Ok(QueryResult::Id(res.pop().unwrap()))
}

fn delete_one(tx: Box<dyn Transaction>, q: DeleteRecord) -> InterpretationResult<QueryResult> {
    // We need to ensure that we have a record finder, else we delete everything (conversion to empty filter).
    let finder = match q.where_ {
        Some(f) => Ok(f),
        None => Err(InterpreterError::InterpretationError(
            "No record finder specified for delete record operation. Aborting.".to_owned(),
        )),
    }?;

    let res = tx.delete_records(q.model, Filter::from(finder))?;

    Ok(QueryResult::Count(res))
}

fn update_many(tx: Box<dyn Transaction>, q: UpdateManyRecords) -> InterpretationResult<QueryResult> {
    let res = tx.update_records(q.model, q.filter, WriteArgs::new(q.non_list_args, q.list_args))?;

    Ok(QueryResult::Count(res.len()))
}

fn delete_many(tx: Box<dyn Transaction>, q: DeleteManyRecords) -> InterpretationResult<QueryResult> {
    let res = tx.delete_records(q.model, q.filter)?;

    Ok(QueryResult::Count(res))
}

fn connect(tx: Box<dyn Transaction>, q: ConnectRecords) -> InterpretationResult<QueryResult> {
    tx.connect(
        q.relation_field,
        &q.parent.expect("Expected parent record ID to be set for connect"),
        &q.child.expect("Expected child record ID to be set for connect"),
    )?;

    Ok(QueryResult::Unit)
}

fn disconnect(tx: Box<dyn Transaction>, q: DisconnectRecords) -> InterpretationResult<QueryResult> {
    tx.disconnect(
        q.relation_field,
        &q.parent.expect("Expected parent record ID to be set for disconnect"),
        &q.child.expect("Expected child record ID to be set for disconnect"),
    )?;

    Ok(QueryResult::Unit)
}

fn set(tx: Box<dyn Transaction>, q: SetRecords) -> InterpretationResult<QueryResult> {
    tx.set(
        q.relation_field,
        q.parent.expect("Expected parent record ID to be set for set"),
        q.wheres,
    )?;

    Ok(QueryResult::Unit)
}

fn reset(_tx: Box<dyn Transaction>, _q: ResetData) -> InterpretationResult<QueryResult> {
    unimplemented!()
}
