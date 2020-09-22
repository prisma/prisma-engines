use super::Visitor;
use crate::prelude::Query;
use crate::{
    ast::{
        Column, Expression, ExpressionKind, Insert, IntoRaw, Merge, OnConflict, Order, Ordering, Row, Table, TableType,
        Values,
    },
    error::{Error, ErrorKind},
    prelude::Average,
    visitor, Value,
};
use std::{convert::TryFrom, fmt::Write, iter};

pub struct Mssql<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
    order_by_set: bool,
}

impl<'a> Mssql<'a> {
    fn visit_returning(&mut self, columns: Vec<Column<'a>>) -> visitor::Result {
        let cols: Vec<_> = columns.into_iter().map(|c| c.table("Inserted")).collect();

        self.write(" OUTPUT ")?;

        let len = cols.len();
        for (i, value) in cols.into_iter().enumerate() {
            self.visit_column(value)?;

            if i < (len - 1) {
                self.write(",")?;
            }
        }

        Ok(())
    }
}

impl<'a> Visitor<'a> for Mssql<'a> {
    const C_BACKTICK_OPEN: &'static str = "[";
    const C_BACKTICK_CLOSE: &'static str = "]";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<crate::ast::Query<'a>>,
    {
        let mut this = Mssql {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
            order_by_set: false,
        };

        Mssql::visit_query(&mut this, query.into())?;

        Ok((this.query, this.parameters))
    }

    fn write<D: std::fmt::Display>(&mut self, s: D) -> visitor::Result {
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value)
    }

    /// A point to modify an incoming query to make it compatible with the
    /// SQL Server.
    fn compatibility_modifications(&self, query: Query<'a>) -> Query<'a> {
        match query {
            // Finding possible `(a, b) (NOT) IN (SELECT x, y ...)` comparisons,
            // and replacing them with common table expressions.
            Query::Select(select) => select.convert_tuple_selects_to_ctes(&mut 0).into(),
            // Replacing the `ON CONFLICT DO NOTHING` clause with a `MERGE` statement.
            Query::Insert(insert) => match insert.on_conflict {
                Some(OnConflict::DoNothing) => Merge::try_from(*insert).unwrap().into(),
                _ => Query::Insert(insert),
            },
            _ => query,
        }
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), false)?;
            }
            (left_kind, right_kind) => {
                self.visit_expression(Expression::from(left_kind))?;
                self.write(" = ")?;
                self.visit_expression(Expression::from(right_kind))?;
            }
        }

        Ok(())
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        match (left.kind, right.kind) {
            // we can't compare with tuples, so we'll convert it to an AND
            (ExpressionKind::Row(left), ExpressionKind::Row(right)) => {
                self.visit_multiple_tuple_comparison(left, Values::from(iter::once(right)), true)?;
            }
            (left_kind, right_kind) => {
                self.visit_expression(Expression::from(left_kind))?;
                self.write(" <> ")?;
                self.visit_expression(Expression::from(right_kind))?;
            }
        }

        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match value {
            Value::Integer(i) => i.map(|i| self.write(i)),
            Value::Real(r) => r.map(|r| self.write(r)),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Enum(e) => e.map(|e| self.write(e)),
            Value::Bytes(b) => b.map(|b| self.write(format!("0x{}", hex::encode(b)))),
            Value::Boolean(b) => b.map(|b| self.write(if b { 1 } else { 0 })),
            Value::Char(c) => c.map(|c| self.write(format!("'{}'", c))),
            #[cfg(feature = "json-1")]
            Value::Json(j) => j.map(|j| self.write(format!("'{}'", serde_json::to_string(&j).unwrap()))),
            #[cfg(all(feature = "array", feature = "postgresql"))]
            Value::Array(_) => {
                let msg = "Arrays are not supported in T-SQL.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                Err(builder.build())?
            }
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(uuid) => uuid.map(|uuid| {
                let s = format!("CONVERT(uniqueidentifier, N'{}')", uuid.to_hyphenated().to_string());
                self.write(s)
            }),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => dt.map(|dt| {
                let s = format!("CONVERT(datetimeoffset, N'{}')", dt.to_rfc3339());
                self.write(s)
            }),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(date) => date.map(|date| {
                let s = format!("CONVERT(date, N'{}')", date);
                self.write(s)
            }),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(time) => time.map(|time| {
                let s = format!("CONVERT(time, N'{}')", time);
                self.write(s)
            }),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> visitor::Result {
        let add_ordering = |this: &mut Self| {
            if !this.order_by_set {
                this.write(" ORDER BY ")?;
                this.visit_ordering(Ordering::new(vec![((1.raw().into(), None))]))?;
            }

            Ok::<(), crate::error::Error>(())
        };

        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, Some(offset)) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)?;
                self.write(" ROWS")
            }
            (Some(limit), None) => {
                add_ordering(self)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(Value::from(0))?;
                self.write(" ROWS FETCH NEXT ")?;
                self.visit_parameterized(limit)?;
                self.write(" ROWS ONLY")
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        self.write("INSERT")?;

        if let Some(table) = insert.table {
            self.write(" INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" DEFAULT VALUES")?;
                } else {
                    self.write(" ")?;
                    self.visit_row(Row::from(insert.columns))?;

                    if let Some(returning) = insert.returning {
                        self.visit_returning(returning)?;
                    }

                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                self.write(" ")?;
                self.visit_row(Row::from(insert.columns))?;

                if let Some(returning) = insert.returning {
                    self.visit_returning(returning)?;
                }

                self.write(" VALUES ")?;

                let values_len = values.len();
                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(",")?;
                    }
                }
            }
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        Ok(())
    }

    fn visit_merge(&mut self, merge: Merge<'a>) -> visitor::Result {
        self.write("MERGE INTO ")?;
        self.visit_table(merge.table, true)?;

        self.write(" USING ")?;

        let base_query = merge.using.base_query;
        self.surround_with("(", ")", |ref mut s| s.visit_query(base_query))?;

        self.write(" AS ")?;
        self.visit_table(merge.using.as_table, false)?;

        self.write(" ")?;
        self.visit_row(Row::from(merge.using.columns))?;
        self.write(" ON ")?;
        self.visit_conditions(merge.using.on_conditions)?;

        if let Some(query) = merge.when_not_matched {
            self.write(" WHEN NOT MATCHED THEN ")?;
            self.visit_query(query)?;
        }

        if let Some(columns) = merge.returning {
            self.visit_returning(columns)?;
        }

        self.write(";")?;

        Ok(())
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.write("@P")?;
        self.write(self.parameters.len())
    }

    fn visit_aggregate_to_string(&mut self, value: crate::ast::Expression<'a>) -> visitor::Result {
        self.write("STRING_AGG")?;
        self.surround_with("(", ")", |ref mut se| {
            se.visit_expression(value)?;
            se.write(",")?;
            se.write("\",\"")
        })
    }

    // MSSQL doesn't support tuples, we do AND/OR.
    fn visit_multiple_tuple_comparison(&mut self, left: Row<'a>, right: Values<'a>, negate: bool) -> visitor::Result {
        let row_len = left.len();
        let values_len = right.len();

        if negate {
            self.write("NOT ")?;
        }

        self.surround_with("(", ")", |this| {
            for (i, row) in right.into_iter().enumerate() {
                this.surround_with("(", ")", |se| {
                    let row_and_vals = left.values.clone().into_iter().zip(row.values.into_iter());

                    for (j, (expr, val)) in row_and_vals.enumerate() {
                        se.visit_expression(expr)?;
                        se.write(" = ")?;
                        se.visit_expression(val)?;

                        if j < row_len - 1 {
                            se.write(" AND ")?;
                        }
                    }

                    Ok(())
                })?;

                if i < values_len - 1 {
                    this.write(" OR ")?;
                }
            }

            Ok(())
        })
    }

    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> visitor::Result {
        let len = ordering.0.len();

        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            let direction = ordering.map(|dir| match dir {
                Order::Asc => " ASC",
                Order::Desc => " DESC",
            });

            self.visit_expression(value)?;
            self.write(direction.unwrap_or(""))?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        self.order_by_set = true;

        Ok(())
    }

    /// A database table identifier
    fn visit_table(&mut self, table: Table<'a>, include_alias: bool) -> visitor::Result {
        match table.typ {
            TableType::Table(table_name) => self.delimited_identifiers(&[&*table_name])?,
            TableType::Values(values) => self.visit_values(values)?,
            TableType::Query(select) => self.surround_with("(", ")", |ref mut s| s.visit_select(select))?,
        };

        if include_alias {
            if let Some(alias) = table.alias {
                self.write(" AS ")?;

                self.delimited_identifiers(&[&*alias])?;
            };
        }

        Ok(())
    }

    fn visit_average(&mut self, avg: Average<'a>) -> visitor::Result {
        self.write("AVG")?;

        // SQL Server will average as an integer, so average of 0 an 1 would be
        // 0, if we don't convert the value to a decimal first.
        self.surround_with("(", ")", |ref mut s| {
            s.write("CONVERT")?;

            s.surround_with("(", ")", |ref mut s| {
                s.write("DECIMAL(32,16),")?;
                s.visit_column(avg.column)
            })
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::*,
        val,
        visitor::{Mssql, Visitor},
    };
    use indoc::indoc;

    fn expected_values<'a, T>(sql: &'static str, params: Vec<T>) -> (String, Vec<Value<'a>>)
    where
        T: Into<Value<'a>>,
    {
        (String::from(sql), params.into_iter().map(|p| p.into()).collect())
    }

    fn default_params<'a>(mut additional: Vec<Value<'a>>) -> Vec<Value<'a>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_select_1() {
        let expected = expected_values("SELECT @P1", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT @P1 AS [test]", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT @P1 AS [test]";
        let query = Select::default().value(val!(Value::Integer(None)).alias("test"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::Integer(None)], params);
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT [musti].* FROM [musti]";
        let query = Select::from_table("musti");
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_in_values() {
        use crate::{col, values};

        let expected_sql =
            "SELECT [test].* FROM [test] WHERE (([id1] = @P1 AND [id2] = @P2) OR ([id1] = @P3 AND [id2] = @P4))";

        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_not_in_values() {
        use crate::{col, values};

        let expected_sql =
            "SELECT [test].* FROM [test] WHERE NOT (([id1] = @P1 AND [id2] = @P2) OR ([id1] = @P3 AND [id2] = @P4))";

        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).not_in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_in_values_singular() {
        let mut cols = Row::new();
        cols.push(Column::from("id1"));

        let mut vals = Values::new(vec![]);

        {
            let mut row1 = Row::new();
            row1.push(1);

            let mut row2 = Row::new();
            row2.push(2);

            vals.push(row1);
            vals.push(row2);
        }

        let query = Select::from_table("test").so_that(cols.in_selection(vals));
        let (sql, params) = Mssql::build(query).unwrap();
        let expected_sql = "SELECT [test].* FROM [test] WHERE [id1] IN (@P1,@P2)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT [musti].* FROM [musti] ORDER BY [foo], [baz] ASC, [bar] DESC";
        let query = Select::from_table("musti")
            .order_by("foo")
            .order_by("baz".ascend())
            .order_by("bar".descend());
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT [paw], [nose] FROM [musti]";
        let query = Select::from_table(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] = @P1", vec!["meow"]);

        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["%meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".begins_with("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_begins_with("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values("SELECT [naukio].* FROM [naukio] WHERE [word] LIKE @P1", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".ends_into("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT [naukio].* FROM [naukio] WHERE [word] NOT LIKE @P1",
            vec!["%meow"],
        );

        let query = Select::from_table("naukio").so_that("word".not_ends_into("meow"));
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE ([word] = @P1 AND [age] < @P2 AND [paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10)).and("paw".equals("warm"));
        let query = Select::from_table("naukio").so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE ([word] = @P1 AND ([age] < @P2 AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = "word".equals("meow").and("age".less_than(10).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql = "SELECT [naukio].* FROM [naukio] WHERE (([word] = @P1 OR [age] < @P2) AND [paw] = @P3)";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = "word".equals("meow").or("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([word] = @P1 OR [age] < @P2) AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"))
            .not();

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT [naukio].* FROM [naukio] WHERE (NOT (([word] = @P1 OR [age] < @P2) AND [paw] = @P3))";

        let expected_params = vec![Value::text("meow"), Value::integer(10), Value::text("warm")];

        let conditions = ConditionTree::not("word".equals("meow").or("age".less_than(10)).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql = "SELECT [users].* FROM [users] INNER JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] INNER JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table("users").inner_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql = "SELECT [users].* FROM [users] LEFT JOIN [posts] ON [users].[id] = [posts].[user_id]";

        let query = Select::from_table("users")
            .left_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT [users].* FROM [users] LEFT JOIN [posts] ON ([users].[id] = [posts].[user_id] AND [posts].[published] = @P1)";

        let query = Select::from_table("users").left_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::boolean(true),]), params);
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT [bar] AS [foo] FROM [meow]";
        let query = Select::from_table("meow").column(Column::new("bar").alias("foo"));
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_limit_with_no_offset() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar").column("foo").order_by("id").limit(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(0), Value::integer(10)], params);
    }

    #[test]
    fn test_offset_no_limit() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS";
        let query = Select::from_table("bar").column("foo").order_by("id").offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10)], params);
    }

    #[test]
    fn test_limit_with_offset() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY [id] OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar")
            .column("foo")
            .order_by("id")
            .limit(9)
            .offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10), Value::integer(9)], params);
    }

    #[test]
    fn test_limit_with_offset_no_given_order() {
        let expected_sql = "SELECT [foo] FROM [bar] ORDER BY 1 OFFSET @P1 ROWS FETCH NEXT @P2 ROWS ONLY";
        let query = Select::from_table("bar").column("foo").limit(9).offset(10);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(10), Value::integer(9)], params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Mssql::build(Select::default().value(Value::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Mssql::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Mssql::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Mssql::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Mssql::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();

        assert_eq!("SELECT 0x010203", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Mssql::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());

        let (sql, params) = Mssql::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT 0", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Mssql::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "json-1")]
    fn test_raw_json() {
        let (sql, params) = Mssql::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid-0_8")]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Mssql::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(
            format!(
                "SELECT CONVERT(uniqueidentifier, N'{}')",
                uuid.to_hyphenated().to_string()
            ),
            sql
        );

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono-0_4")]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Mssql::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT CONVERT(datetimeoffset, N'{}')", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_single_insert() {
        let insert = Insert::single_into("foo").value("bar", "lol").value("wtf", "meow");
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] ([bar],[wtf]) VALUES (@P1,@P2)", sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_default() {
        let insert = Insert::single_into("foo");
        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] DEFAULT VALUES", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_returning_insert() {
        let insert = Insert::single_into("foo").value("bar", "lol");
        let (sql, params) = Mssql::build(Insert::from(insert).returning(vec!["bar"])).unwrap();

        assert_eq!("INSERT INTO [foo] ([bar]) OUTPUT [Inserted].[bar] VALUES (@P1)", sql);

        assert_eq!(vec![Value::from("lol")], params);
    }

    #[test]
    fn test_multi_insert() {
        let insert = Insert::multi_into("foo", vec!["bar", "wtf"])
            .values(vec!["lol", "meow"])
            .values(vec!["omg", "hey"]);

        let (sql, params) = Mssql::build(insert).unwrap();

        assert_eq!("INSERT INTO [foo] ([bar],[wtf]) VALUES (@P1,@P2),(@P3,@P4)", sql);

        assert_eq!(
            vec![
                Value::from("lol"),
                Value::from("meow"),
                Value::from("omg"),
                Value::from("hey")
            ],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique() {
        let table = Table::from("foo").add_unique_index("bar");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_single_unique_with_default() {
        let unique_column = Column::from("bar").default("purr");
        let table = Table::from("foo").add_unique_index(unique_column);

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON [foo].[bar] = @P2
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    #[cfg(feature = "mssql")]
    fn test_single_insert_conflict_with_returning_clause() {
        let table = Table::from("foo").add_unique_index("bar");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let insert = insert
            .on_conflict(OnConflict::DoNothing)
            .returning(vec![("foo", "bar"), ("foo", "wtf")]);

        let (sql, params) = Mssql::build(insert).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON [dual].[bar] = [foo].[bar]
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf])
            OUTPUT [Inserted].[bar],[Inserted].[wtf];
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques() {
        let table = Table::from("foo").add_unique_index("bar").add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_two_uniques_with_default() {
        let unique_column = Column::from("bar").default("purr");

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn generated_unique_defaults_should_not_be_part_of_the_join_when_value_is_not_provided() {
        let unique_column = Column::from("bar").default("purr");
        let default_column = Column::from("lol").default(DefaultValue::Generated);

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn with_generated_unique_defaults_the_value_should_be_part_of_the_join() {
        let unique_column = Column::from("bar").default("purr");
        let default_column = Column::from("lol").default(DefaultValue::Generated);

        let table = Table::from("foo")
            .add_unique_index(unique_column)
            .add_unique_index(default_column)
            .add_unique_index("wtf");

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "wtf"), "meow")
            .value(("foo", "lol"), "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON ([foo].[bar] = @P3 OR [dual].[lol] = [foo].[lol] OR [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);

        assert_eq!(
            vec![Value::from("meow"), Value::from("hiss"), Value::from("purr")],
            params
        );
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique() {
        let table = Table::from("foo").add_unique_index(vec!["bar", "wtf"]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "bar"), "lol")
            .value(("foo", "wtf"), "meow")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [bar], @P2 AS [wtf]) AS [dual] ([bar],[wtf])
            ON ([dual].[bar] = [foo].[bar] AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([bar],[wtf]) VALUES ([dual].[bar],[dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("lol"), Value::from("meow")], params);
    }

    #[test]
    fn test_single_insert_conflict_do_nothing_compound_unique_with_default() {
        let bar = Column::from("bar").default("purr");
        let wtf = Column::from("wtf");

        let table = Table::from("foo").add_unique_index(vec![bar, wtf]);
        let insert: Insert<'_> = Insert::single_into(table).value(("foo", "wtf"), "meow").into();
        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf]) AS [dual] ([wtf])
            ON ([foo].[bar] = @P2 AND [dual].[wtf] = [foo].[wtf])
            WHEN NOT MATCHED THEN
            INSERT ([wtf]) VALUES ([dual].[wtf]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(vec![Value::from("meow"), Value::from("purr")], params);
    }

    #[test]
    fn one_generated_value_in_compound_unique_removes_the_whole_index_from_the_join() {
        let bar = Column::from("bar").default("purr");
        let wtf = Column::from("wtf");

        let omg = Column::from("omg").default(DefaultValue::Generated);
        let lol = Column::from("lol");

        let table = Table::from("foo")
            .add_unique_index(vec![bar, wtf])
            .add_unique_index(vec![omg, lol]);

        let insert: Insert<'_> = Insert::single_into(table)
            .value(("foo", "wtf"), "meow")
            .value(("foo", "lol"), "hiss")
            .into();

        let (sql, params) = Mssql::build(insert.on_conflict(OnConflict::DoNothing)).unwrap();

        let expected_sql = indoc!(
            "
            MERGE INTO [foo]
            USING (SELECT @P1 AS [wtf], @P2 AS [lol]) AS [dual] ([wtf],[lol])
            ON (([foo].[bar] = @P3 AND [dual].[wtf] = [foo].[wtf]) OR (1=0 AND [dual].[lol] = [foo].[lol]))
            WHEN NOT MATCHED THEN
            INSERT ([wtf],[lol]) VALUES ([dual].[wtf],[dual].[lol]);
        "
        );

        assert_eq!(expected_sql.replace('\n', " ").trim(), sql);
        assert_eq!(
            vec![Value::from("meow"), Value::from("hiss"), Value::from("purr")],
            params
        );
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT [bar] FROM [test]";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT @P1 FROM [test2]), [bar] FROM [test]";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_from() {
        let expected_sql = "SELECT [foo].*, [bar].[a] FROM [foo], (SELECT [a] FROM [baz]) AS [bar]";
        let query = Select::default()
            .and_from("foo")
            .and_from(Table::from(Select::from_table("baz").column("a")).alias("bar"))
            .value(Table::from("foo").asterisk())
            .column(("bar", "a"));

        let (sql, _) = Mssql::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_cte_conversion_top_level_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let query = Select::from_table("A").so_that(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2)], params);
    }

    #[test]
    fn test_cte_conversion_top_level_not_in() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE [A].[x] NOT IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let query = Select::from_table("A").so_that(row.not_in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::integer(1), Value::integer(2)], params);
    }

    #[test]
    fn test_cte_conversion_in_a_tree_top_level() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3
            AND [A].[z] = @P4
            AND [A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y]))"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);

        let query = Select::from_table("A")
            .so_that(("A", "y").equals("bar"))
            .and_where(("A", "z").equals("foo"))
            .and_where(row.in_selection(inner));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::text("bar"),
                Value::text("foo")
            ],
            params
        );
    }

    #[test]
    fn test_cte_conversion_in_a_tree_nested() {
        let expected_sql = indoc!(
            r#"WITH [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b])
            SELECT [A].* FROM [A]
            WHERE ([A].[y] = @P3 OR ([A].[z] = @P4 AND [A].[x] IN
            (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])))"#
        )
        .replace('\n', " ");

        let inner = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let row = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);

        let cond = ("A", "y")
            .equals("bar")
            .or(("A", "z").equals("foo").and(row.in_selection(inner)));

        let query = Select::from_table("A").so_that(cond);
        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::text("bar"),
                Value::text("foo")
            ],
            params
        );
    }

    #[test]
    fn test_multiple_cte_conversions_in_the_ast() {
        let expected_sql = indoc!(
            r#"WITH
            [cte_0] AS (SELECT @P1 AS [a], @P2 AS [b]),
            [cte_1] AS (SELECT @P3 AS [c], @P4 AS [d])
            SELECT [A].* FROM [A]
            WHERE ([A].[x] IN (SELECT [a] FROM [cte_0] WHERE [b] = [A].[y])
            AND [A].[u] NOT IN (SELECT [c] FROM [cte_1] WHERE [d] = [A].[z]))"#
        )
        .replace('\n', " ");

        let cte_0 = Select::default().value(val!(1).alias("a")).value(val!(2).alias("b"));
        let cte_1 = Select::default().value(val!(3).alias("c")).value(val!(4).alias("d"));
        let row_0 = Row::from(vec![col!(("A", "x")), col!(("A", "y"))]);
        let row_1 = Row::from(vec![col!(("A", "u")), col!(("A", "z"))]);

        let query = Select::from_table("A")
            .so_that(row_0.in_selection(cte_0))
            .and_where(row_1.not_in_selection(cte_1));

        let (sql, params) = Mssql::build(query).unwrap();

        assert_eq!(expected_sql, sql);

        assert_eq!(
            vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
                Value::integer(4)
            ],
            params
        );
    }
}
