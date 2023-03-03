use sql_schema_describer as sql;

pub(crate) struct SqliteIntrospectionFlavour;

impl super::IntrospectionFlavour for SqliteIntrospectionFlavour {
    fn keep_previous_scalar_field_arity(&self, next: sql::ColumnWalker<'_>) -> bool {
        next.is_in_view() && next.column_type().arity.is_nullable()
    }
}
