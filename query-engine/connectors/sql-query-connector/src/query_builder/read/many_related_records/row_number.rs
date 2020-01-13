use super::*;

pub struct ManyRelatedRecordsWithRowNumber;

use crate::{query_builder::read, ordering::Ordering};
use prisma_models::{SelectedFields, sql_ext::AsColumn};
use quaint::ast::*;

impl ManyRelatedRecordsQueryBuilder for ManyRelatedRecordsWithRowNumber {
    fn with_pagination(base: ManyRelatedRecordsBaseQuery) -> Query {
        let conditions = read::relation_in_selection(base.from_field, base.from_record_ids)
                .and(base.condition)
                .and(base.cursor);

        let mut base_query = base.query.so_that(conditions);

        if let Some(order_by) = &base.order_directions.primary_order_by {
            let column = order_by.field.as_column();

            if !base.columns.contains(&column) {
                base_query = base_query.column(order_by.field.as_column());
            }
        }

        let order_columns = Ordering::aliased_internal(
            Self::BASE_TABLE_ALIAS,
            Self::BASE_TABLE_ALIAS,
            SelectedFields::RELATED_MODEL_ALIAS,
            base.order_directions,
        );

        let row_number_part: Function = order_columns
            .into_iter()
            .fold(row_number(), |acc, ord| acc.order_by(ord))
            .partition_by((Self::BASE_TABLE_ALIAS, SelectedFields::PARENT_MODEL_ALIAS))
            .into();

        let with_row_numbers = Select::from_table(Table::from(base_query).alias(Self::BASE_TABLE_ALIAS))
            .value(Table::from(Self::BASE_TABLE_ALIAS).asterisk())
            .value(row_number_part.alias(Self::ROW_NUMBER_ALIAS));

        Select::from_table(Table::from(with_row_numbers).alias(Self::ROW_NUMBER_TABLE_ALIAS))
            .value(Table::from(Self::ROW_NUMBER_TABLE_ALIAS).asterisk())
            .so_that(Self::ROW_NUMBER_ALIAS.between(base.window_limits.0 as i64, base.window_limits.1 as i64))
            .into()
    }
}
