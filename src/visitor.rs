//! Visitors for reading an abstract SQL syntax tree, generating the query and
//! gathering parameters in the right order.
//!
//! The visitor module should not know how to construct an AST, just how to read
//! one. Everything related to the tree generation is in the
//! [ast](../ast/index.html) module.
//!
//! For prelude, all important imports are in `quaint::visitor::*`;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;

pub use self::mssql::Mssql;
pub use self::mysql::Mysql;
pub use self::postgres::Postgres;
pub use self::sqlite::Sqlite;

use crate::ast::*;
use std::fmt;

pub type Result = crate::Result<()>;

/// A function travelling through the query AST, building the final query string
/// and gathering parameters sent to the database together with the query.
pub trait Visitor<'a> {
    /// Opening backtick character to surround identifiers, such as column and table names.
    const C_BACKTICK_OPEN: &'static str;
    /// Closing backtick character to surround identifiers, such as column and table names.
    const C_BACKTICK_CLOSE: &'static str;
    /// Wildcard character to be used in `LIKE` queries.
    const C_WILDCARD: &'static str;

    /// Convert the given `Query` to an SQL string and a vector of parameters.
    /// When certain parameters are replaced with the `C_PARAM` character in the
    /// query, the vector should contain the parameter value in the right position.
    ///
    /// The point of entry for visiting query ASTs.
    ///
    /// ```
    /// # use quaint::{ast::*, visitor::*, error::Error};
    /// # fn main() -> Result {
    /// let query = Select::from_table("cats");
    /// let (sqlite, _) = Sqlite::build(query.clone())?;
    /// let (psql, _) = Postgres::build(query.clone())?;
    /// let (mysql, _) = Mysql::build(query.clone())?;
    /// let (mssql, _) = Mssql::build(query.clone())?;
    ///
    /// assert_eq!("SELECT `cats`.* FROM `cats`", sqlite);
    /// assert_eq!("SELECT \"cats\".* FROM \"cats\"", psql);
    /// assert_eq!("SELECT `cats`.* FROM `cats`", mysql);
    /// assert_eq!("SELECT [cats].* FROM [cats]", mssql);
    /// # Ok(())
    /// # }
    /// ```
    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>;

    /// Write to the query.
    fn write<D: fmt::Display>(&mut self, s: D) -> Result;

    fn surround_with<F>(&mut self, begin: &str, end: &str, f: F) -> Result
    where
        F: FnOnce(&mut Self) -> Result,
    {
        self.write(begin)?;
        f(self)?;
        self.write(end)
    }

    /// When called, the visitor decided to not render the parameter into the query,
    /// replacing it with the `C_PARAM`, calling `add_parameter` with the replaced value.
    fn add_parameter(&mut self, value: Value<'a>);

    /// The `LIMIT` and `OFFSET` statement in the query
    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> Result;

    /// A walk through an `INSERT` statement
    fn visit_insert(&mut self, insert: Insert<'a>) -> Result;

    /// What to use to substitute a parameter in the query.
    fn parameter_substitution(&mut self) -> Result;

    /// What to use to substitute a parameter in the query.
    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> Result;

    /// Visit a non-parameterized value.
    fn visit_raw_value(&mut self, value: Value<'a>) -> Result;

    /// A visit to a value we parameterize
    fn visit_parameterized(&mut self, value: Value<'a>) -> Result {
        self.add_parameter(value);
        self.parameter_substitution()
    }

    /// The join statements in the query
    fn visit_joins(&mut self, joins: Vec<Join<'a>>) -> Result {
        for j in joins {
            match j {
                Join::Inner(data) => {
                    self.write(" INNER JOIN ")?;
                    self.visit_join_data(data)?;
                }
                Join::Left(data) => {
                    self.write(" LEFT JOIN ")?;
                    self.visit_join_data(data)?;
                }
                Join::Right(data) => {
                    self.write(" RIGHT JOIN ")?;
                    self.visit_join_data(data)?;
                }
                Join::Full(data) => {
                    self.write(" FULL JOIN ")?;
                    self.visit_join_data(data)?;
                }
            }
        }

        Ok(())
    }

    fn visit_join_data(&mut self, data: JoinData<'a>) -> Result {
        self.visit_table(data.table, true)?;
        self.write(" ON ")?;
        self.visit_conditions(data.conditions)
    }

    /// A walk through a `SELECT` statement
    fn visit_select(&mut self, select: Select<'a>) -> Result {
        self.write("SELECT ")?;

        if select.distinct {
            self.write("DISTINCT ")?;
        }

        if !select.tables.is_empty() {
            if select.columns.is_empty() {
                for (i, table) in select.tables.iter().enumerate() {
                    if i > 0 {
                        self.write(", ")?;
                    }

                    match table.typ {
                        TableType::Query(_) | TableType::Values(_) => match table.alias {
                            Some(ref alias) => {
                                self.surround_with(Self::C_BACKTICK_OPEN, Self::C_BACKTICK_CLOSE, |ref mut s| {
                                    s.write(alias)
                                })?;
                                self.write(".*")?;
                            }
                            None => self.write("*")?,
                        },
                        TableType::Table(_) => match table.alias.clone() {
                            Some(ref alias) => {
                                self.surround_with(Self::C_BACKTICK_OPEN, Self::C_BACKTICK_CLOSE, |ref mut s| {
                                    s.write(alias)
                                })?;
                                self.write(".*")?;
                            }
                            None => {
                                self.visit_table(*table.clone(), false)?;
                                self.write(".*")?;
                            }
                        },
                    }
                }
            } else {
                self.visit_columns(select.columns)?;
            }

            self.write(" FROM ")?;

            for (i, table) in select.tables.into_iter().enumerate() {
                if i > 0 {
                    self.write(", ")?;
                }

                self.visit_table(*table, true)?;
            }

            if !select.joins.is_empty() {
                self.visit_joins(select.joins)?;
            }

            if let Some(conditions) = select.conditions {
                self.write(" WHERE ")?;
                self.visit_conditions(conditions)?;
            }
            if !select.grouping.is_empty() {
                self.write(" GROUP BY ")?;
                self.visit_grouping(select.grouping)?;
            }
            if let Some(conditions) = select.having {
                self.write(" HAVING ")?;
                self.visit_conditions(conditions)?;
            }
            if !select.ordering.is_empty() {
                self.write(" ORDER BY ")?;
                self.visit_ordering(select.ordering)?;
            }

            self.visit_limit_and_offset(select.limit, select.offset)?;
        } else if select.columns.is_empty() {
            self.write(" *")?;
        } else {
            self.visit_columns(select.columns)?;
        }

        Ok(())
    }

    /// A walk through an `UPDATE` statement
    fn visit_update(&mut self, update: Update<'a>) -> Result {
        self.write("UPDATE ")?;
        self.visit_table(update.table, true)?;

        {
            self.write(" SET ")?;
            let pairs = update.columns.into_iter().zip(update.values.into_iter());
            let len = pairs.len();

            for (i, (key, value)) in pairs.enumerate() {
                self.visit_column(key)?;
                self.write(" = ")?;
                self.visit_expression(value)?;

                if i < (len - 1) {
                    self.write(", ")?;
                }
            }
        }

        if let Some(conditions) = update.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        Ok(())
    }

    /// A walk through an `DELETE` statement
    fn visit_delete(&mut self, delete: Delete<'a>) -> Result {
        self.write("DELETE FROM ")?;
        self.visit_table(delete.table, true)?;

        if let Some(conditions) = delete.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        Ok(())
    }

    /// A helper for delimiting an identifier, surrounding every part with `C_BACKTICK`
    /// and delimiting the values with a `.`
    fn delimited_identifiers(&mut self, parts: &[&str]) -> Result {
        let len = parts.len();

        for (i, parts) in parts.iter().enumerate() {
            self.surround_with(Self::C_BACKTICK_OPEN, Self::C_BACKTICK_CLOSE, |ref mut s| {
                s.write(parts)
            })?;

            if i < (len - 1) {
                self.write(".")?;
            }
        }

        Ok(())
    }

    /// A walk through a complete `Query` statement
    fn visit_query(&mut self, query: Query<'a>) -> Result {
        match query {
            Query::Select(select) => self.visit_select(*select),
            Query::Insert(insert) => self.visit_insert(*insert),
            Query::Update(update) => self.visit_update(*update),
            Query::Delete(delete) => self.visit_delete(*delete),
            Query::Union(union) => self.visit_union(union),
            Query::Raw(string) => self.write(string),
        }
    }

    /// A walk through a union of `SELECT` statements
    fn visit_union(&mut self, mut ua: Union<'a>) -> Result {
        let len = ua.selects.len();
        let mut types = ua.types.drain(0..);

        for (i, sel) in ua.selects.into_iter().enumerate() {
            self.surround_with("(", ")", |ref mut se| se.visit_select(sel))?;

            if i < (len - 1) {
                let typ = types.next().unwrap();

                self.write(" ")?;
                self.write(typ)?;
                self.write(" ")?;
            }
        }

        Ok(())
    }

    /// The selected columns
    fn visit_columns(&mut self, columns: Vec<Expression<'a>>) -> Result {
        let len = columns.len();

        for (i, column) in columns.into_iter().enumerate() {
            self.visit_expression(column)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_operation(&mut self, op: SqlOp<'a>) -> Result {
        match op {
            SqlOp::Add(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_expression(left)?;
                se.write(" + ")?;
                se.visit_expression(right)
            }),
            SqlOp::Sub(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_expression(left)?;
                se.write(" - ")?;
                se.visit_expression(right)
            }),
            SqlOp::Mul(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_expression(left)?;
                se.write(" * ")?;
                se.visit_expression(right)
            }),
            SqlOp::Div(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_expression(left)?;
                se.write(" / ")?;
                se.visit_expression(right)
            }),
            SqlOp::Rem(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_expression(left)?;
                se.write(" % ")?;
                se.visit_expression(right)
            }),
        }
    }

    /// A visit to a value used in an expression
    fn visit_expression(&mut self, value: Expression<'a>) -> Result {
        match value.kind {
            ExpressionKind::Value(value) => self.visit_expression(*value)?,
            ExpressionKind::ConditionTree(tree) => self.visit_conditions(tree)?,
            ExpressionKind::Compare(compare) => self.visit_compare(compare)?,
            ExpressionKind::Parameterized(val) => self.visit_parameterized(val)?,
            ExpressionKind::RawValue(val) => self.visit_raw_value(val.0)?,
            ExpressionKind::Column(column) => self.visit_column(*column)?,
            ExpressionKind::Row(row) => self.visit_row(row)?,
            ExpressionKind::Select(select) => self.surround_with("(", ")", |ref mut s| s.visit_select(*select))?,
            ExpressionKind::Function(function) => self.visit_function(function)?,
            ExpressionKind::Op(op) => self.visit_operation(*op)?,
            ExpressionKind::Values(values) => self.visit_values(*values)?,
            ExpressionKind::Asterisk(table) => match table {
                Some(table) => {
                    self.visit_table(*table, false)?;
                    self.write(".*")?
                }
                None => self.write("*")?,
            },
        }

        if let Some(alias) = value.alias {
            self.write(" AS ")?;

            self.delimited_identifiers(&[&*alias])?;
        };

        Ok(())
    }

    fn visit_multiple_tuple_comparison(&mut self, left: Row<'a>, right: Values<'a>, negate: bool) -> Result {
        self.visit_row(left)?;
        self.write(if negate { " NOT IN " } else { " IN " })?;
        self.visit_values(right)
    }

    fn visit_values(&mut self, values: Values<'a>) -> Result {
        self.surround_with("(", ")", |ref mut s| {
            let len = values.len();
            for (i, row) in values.into_iter().enumerate() {
                s.visit_row(row)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }
            Ok(())
        })
    }

    /// A database table identifier
    fn visit_table(&mut self, table: Table<'a>, include_alias: bool) -> Result {
        match table.typ {
            TableType::Table(table_name) => match table.database {
                Some(database) => self.delimited_identifiers(&[&*database, &*table_name])?,
                None => self.delimited_identifiers(&[&*table_name])?,
            },
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

    /// A database column identifier
    fn visit_column(&mut self, column: Column<'a>) -> Result {
        match column.table {
            Some(table) => {
                self.visit_table(table, false)?;
                self.write(".")?;
                self.delimited_identifiers(&[&*column.name])?;
            }
            _ => self.delimited_identifiers(&[&*column.name])?,
        };

        if let Some(alias) = column.alias {
            self.write(" AS ")?;
            self.delimited_identifiers(&[&*alias])?;
        }

        Ok(())
    }

    /// A row of data used as an expression
    fn visit_row(&mut self, row: Row<'a>) -> Result {
        self.surround_with("(", ")", |ref mut s| {
            let len = row.values.len();
            for (i, value) in row.values.into_iter().enumerate() {
                s.visit_expression(value)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }

            Ok(())
        })
    }

    /// A walk through the query conditions
    fn visit_conditions(&mut self, tree: ConditionTree<'a>) -> Result {
        match tree {
            ConditionTree::And(expressions) => self.surround_with("(", ")", |ref mut s| {
                let len = expressions.len();

                for (i, expr) in expressions.into_iter().enumerate() {
                    s.visit_expression(expr)?;

                    if i < (len - 1) {
                        s.write(" AND ")?;
                    }
                }

                Ok(())
            }),
            ConditionTree::Or(expressions) => self.surround_with("(", ")", |ref mut s| {
                let len = expressions.len();

                for (i, expr) in expressions.into_iter().enumerate() {
                    s.visit_expression(expr)?;

                    if i < (len - 1) {
                        s.write(" OR ")?;
                    }
                }

                Ok(())
            }),
            ConditionTree::Not(expression) => self.surround_with("(", ")", |ref mut s| {
                s.write("NOT ")?;
                s.visit_expression(*expression)
            }),
            ConditionTree::Single(expression) => self.visit_expression(*expression),
            ConditionTree::NoCondition => self.write("1=1"),
            ConditionTree::NegativeCondition => self.write("1=0"),
        }
    }

    /// A comparison expression
    fn visit_compare(&mut self, compare: Compare<'a>) -> Result {
        match compare {
            Compare::Equals(left, right) => self.visit_condition_equals(*left, *right),
            Compare::NotEquals(left, right) => self.visit_condition_not_equals(*left, *right),
            Compare::LessThan(left, right) => {
                self.visit_expression(*left)?;
                self.write(" < ")?;
                self.visit_expression(*right)
            }
            Compare::LessThanOrEquals(left, right) => {
                self.visit_expression(*left)?;
                self.write(" <= ")?;
                self.visit_expression(*right)
            }
            Compare::GreaterThan(left, right) => {
                self.visit_expression(*left)?;
                self.write(" > ")?;
                self.visit_expression(*right)
            }
            Compare::GreaterThanOrEquals(left, right) => {
                self.visit_expression(*left)?;
                self.write(" >= ")?;
                self.visit_expression(*right)
            }
            Compare::In(left, right) => match (*left, *right) {
                // To prevent `x IN ()` from happening.
                (
                    _,
                    Expression {
                        kind: ExpressionKind::Row(ref row),
                        ..
                    },
                ) if row.is_empty() => self.write("1=0"),

                // To prevent `x IN ()` from happening.
                (
                    Expression {
                        kind: ExpressionKind::Row(_),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(ref vals),
                        ..
                    },
                ) if vals.row_len() == 0 => self.write("1=0"),

                // Flattening out a row.
                (
                    Expression {
                        kind: ExpressionKind::Row(mut cols),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(vals),
                        ..
                    },
                ) if cols.len() == 1 && vals.row_len() == 1 => {
                    let col = cols.pop().unwrap();
                    let vals = vals.flatten_row().unwrap();

                    self.visit_expression(col)?;
                    self.write(" IN ")?;
                    self.visit_row(vals)
                }

                // No need to do `IN` if right side is only one value,
                (
                    left,
                    Expression {
                        kind: ExpressionKind::Parameterized(pv),
                        ..
                    },
                ) => {
                    self.visit_expression(left)?;
                    self.write(" = ")?;
                    self.visit_parameterized(pv)
                }

                (
                    Expression {
                        kind: ExpressionKind::Row(row),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(values),
                        ..
                    },
                ) => self.visit_multiple_tuple_comparison(row, *values, false),

                // expr IN (..)
                (left, right) => {
                    self.visit_expression(left)?;
                    self.write(" IN ")?;
                    self.visit_expression(right)
                }
            },
            Compare::NotIn(left, right) => match (*left, *right) {
                // To prevent `x NOT IN ()` from happening.
                (
                    _,
                    Expression {
                        kind: ExpressionKind::Row(ref row),
                        ..
                    },
                ) if row.is_empty() => self.write("1=1"),

                // To prevent `x NOT IN ()` from happening.
                (
                    Expression {
                        kind: ExpressionKind::Row(_),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(ref vals),
                        ..
                    },
                ) if vals.row_len() == 0 => self.write("1=1"),

                // Flattening out a row.
                (
                    Expression {
                        kind: ExpressionKind::Row(mut cols),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(vals),
                        ..
                    },
                ) if cols.len() == 1 && vals.row_len() == 1 => {
                    let col = cols.pop().unwrap();
                    let vals = vals.flatten_row().unwrap();

                    self.visit_expression(col)?;
                    self.write(" NOT IN ")?;
                    self.visit_row(vals)
                }

                // No need to do `IN` if right side is only one value,
                (
                    left,
                    Expression {
                        kind: ExpressionKind::Parameterized(pv),
                        ..
                    },
                ) => {
                    self.visit_expression(left)?;
                    self.write(" <> ")?;
                    self.visit_parameterized(pv)
                }

                (
                    Expression {
                        kind: ExpressionKind::Row(row),
                        ..
                    },
                    Expression {
                        kind: ExpressionKind::Values(values),
                        ..
                    },
                ) => self.visit_multiple_tuple_comparison(row, *values, true),

                // expr IN (..)
                (left, right) => {
                    self.visit_expression(left)?;
                    self.write(" NOT IN ")?;
                    self.visit_expression(right)
                }
            },
            Compare::Like(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                )));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotLike(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                )));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::BeginsWith(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!("{}{}", right, Self::C_WILDCARD)));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotBeginsWith(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!("{}{}", right, Self::C_WILDCARD)));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::EndsInto(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!("{}{}", Self::C_WILDCARD, right,)));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotEndsInto(left, right) => {
                self.visit_expression(*left)?;

                self.add_parameter(Value::text(format!("{}{}", Self::C_WILDCARD, right,)));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::Null(column) => {
                self.visit_expression(*column)?;
                self.write(" IS NULL")
            }
            Compare::NotNull(column) => {
                self.visit_expression(*column)?;
                self.write(" IS NOT NULL")
            }
            Compare::Between(val, left, right) => {
                self.visit_expression(*val)?;
                self.write(" BETWEEN ")?;
                self.visit_expression(*left)?;
                self.write(" AND ")?;
                self.visit_expression(*right)
            }
            Compare::NotBetween(val, left, right) => {
                self.visit_expression(*val)?;
                self.write(" NOT BETWEEN ")?;
                self.visit_expression(*left)?;
                self.write(" AND ")?;
                self.visit_expression(*right)
            }
        }
    }

    fn visit_condition_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_condition_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    /// A visit in the `ORDER BY` section of the query
    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> Result {
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

        Ok(())
    }

    /// A visit in the `GROUP BY` section of the query
    fn visit_grouping(&mut self, grouping: Grouping<'a>) -> Result {
        let len = grouping.0.len();

        for (i, value) in grouping.0.into_iter().enumerate() {
            self.visit_expression(value)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_function(&mut self, fun: Function<'a>) -> Result {
        match fun.typ_ {
            FunctionType::RowNumber(fun_rownum) => {
                if fun_rownum.over.is_empty() {
                    self.write("ROW_NUMBER() OVER()")?;
                } else {
                    self.write("ROW_NUMBER() OVER")?;
                    self.surround_with("(", ")", |ref mut s| s.visit_partitioning(fun_rownum.over))?;
                }
            }
            FunctionType::Count(fun_count) => {
                if fun_count.exprs.is_empty() {
                    self.write("COUNT(*)")?;
                } else {
                    self.write("COUNT")?;
                    self.surround_with("(", ")", |ref mut s| s.visit_columns(fun_count.exprs))?;
                }
            }
            FunctionType::AggregateToString(agg) => {
                self.visit_aggregate_to_string(agg.value.as_ref().clone())?;
            }
            FunctionType::Average(avg) => {
                self.write("AVG")?;
                self.surround_with("(", ")", |ref mut s| s.visit_column(avg.column))?;
            }
            FunctionType::Sum(sum) => {
                self.write("SUM")?;
                self.surround_with("(", ")", |ref mut s| s.visit_expression(*sum.expr))?;
            }
            FunctionType::Lower(lower) => {
                self.write("LOWER")?;
                self.surround_with("(", ")", |ref mut s| s.visit_expression(*lower.expression))?;
            }
            FunctionType::Upper(upper) => {
                self.write("UPPER")?;
                self.surround_with("(", ")", |ref mut s| s.visit_expression(*upper.expression))?;
            }
            FunctionType::Minimum(min) => {
                self.write("MIN")?;
                self.surround_with("(", ")", |ref mut s| s.visit_column(min.column))?;
            }
            FunctionType::Maximum(max) => {
                self.write("MAX")?;
                self.surround_with("(", ")", |ref mut s| s.visit_column(max.column))?;
            }
        };

        if let Some(alias) = fun.alias {
            self.write(" AS ")?;
            self.delimited_identifiers(&[&*alias])?;
        }

        Ok(())
    }

    fn visit_partitioning(&mut self, over: Over<'a>) -> Result {
        if !over.partitioning.is_empty() {
            let len = over.partitioning.len();
            self.write("PARTITION BY ")?;

            for (i, partition) in over.partitioning.into_iter().enumerate() {
                self.visit_column(partition)?;

                if i < (len - 1) {
                    self.write(", ")?;
                }
            }

            if !over.ordering.is_empty() {
                self.write(" ")?;
            }
        }

        if !over.ordering.is_empty() {
            self.write("ORDER BY ")?;
            self.visit_ordering(over.ordering)?;
        }

        Ok(())
    }
}
