use crate::{interpreter::InterpretationResult, WriteQuery};
use connector::{TransactionLike, WriteQueryResult};

pub fn execute(_tx: &mut dyn TransactionLike, _write_query: WriteQuery) -> InterpretationResult<WriteQueryResult> {
    unimplemented!()
    // match write_query {
    //     WriteQuery::Root(wq) => self
    //         .write_executor
    //         .execute(self.db_name.clone(), wq)
    //         .map_err(|err| err.into()),

    //     _ => Err(CoreError::UnsupportedFeatureError(
    //         "Attempted to execute nested write query on the root level.".into(),
    //     )),
    // }
}
