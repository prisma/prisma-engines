mod base_query;
mod row_number;
mod union_all;

pub use base_query::*;
pub use row_number::*;
pub use union_all::*;

use crate::{ordering::Ordering, query_builder};
use prisma_models::sql_ext::RelationFieldExt;
use quaint::ast::{Conjuctive, Query};

pub trait ManyRelatedRecordsQueryBuilder {
    const BASE_TABLE_ALIAS: &'static str = "prismaBaseTableAlias";
    const ROW_NUMBER_ALIAS: &'static str = "prismaRowNumberAlias";
    const ROW_NUMBER_TABLE_ALIAS: &'static str = "prismaRowNumberTableAlias";

    fn with_pagination<'a>(base: ManyRelatedRecordsBaseQuery<'a>) -> Query;

    fn without_pagination<'a>(base: ManyRelatedRecordsBaseQuery<'a>) -> Query {
        let columns: Vec<_> = base.from_field.relation_columns(true).collect();

        let conditions = query_builder::conditions(&columns, base.from_record_ids)
            .and(base.condition)
            .and(base.cursor);

        let opposite_columns = base.from_field.opposite_columns(true);
        let order_columns = Ordering::internal(opposite_columns, base.order_directions);

        order_columns
            .into_iter()
            .fold(base.query.so_that(conditions), |acc, ord| acc.order_by(ord))
            .into()
    }

    fn uses_row_number() -> bool;
}
