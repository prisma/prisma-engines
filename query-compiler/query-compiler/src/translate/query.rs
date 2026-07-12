mod read;
mod write;

use std::mem;

use itertools::Itertools;
use query_builder::QueryBuilder;
use query_core::Query;
use query_structure::{PrismaValue, ScalarWriteOperation, WriteOperation};
use read::translate_read_query;
use write::translate_write_query;

use crate::{
    binding,
    expression::{Binding, Expression},
};

use super::TranslateResult;

pub(crate) fn translate_query(query: Query, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    match query {
        Query::Read(rq) => translate_read_query(rq, builder),
        Query::Write(mut wq) => {
            // Extract any side-effectful generator calls from an underlying INSERT (if any) and
            // convert them into bindings.
            let bindings = wq
                .insert_args_mut()
                .iter_mut()
                .enumerate()
                .flat_map(|(row_idx, args)| args.args.iter_mut().map(move |arg| (row_idx, arg)))
                .filter_map(|(row_idx, (name, arg))| {
                    if let WriteOperation::Scalar(ScalarWriteOperation::Set(val @ PrismaValue::GeneratorCall { .. })) =
                        arg
                    {
                        let name = binding::generated(row_idx, name);
                        let val = mem::replace(val, PrismaValue::placeholder(name.clone(), val.r#type()));
                        Some(Binding::new(name, Expression::Value(val)))
                    } else {
                        None
                    }
                })
                .collect_vec();

            if !bindings.is_empty() {
                Ok(Expression::Let {
                    bindings,
                    expr: translate_write_query(wq, builder)?.into(),
                })
            } else {
                translate_write_query(wq, builder)
            }
        }
    }
}
