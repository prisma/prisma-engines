//! Visitors for reading an abstract SQL syntax tree, generating the query and
//! gathering parameters in the right order.
//!
//! The visitor module should not know how to construct an AST, just how to read
//! one. Everything related to the tree generation is in the
//! [ast](../ast/index.html) module.
//!
//! For prelude, all important imports are in `quaint::visitor::*`;
mod mysql;
mod postgres;
mod sqlite;

pub use self::mysql::Mysql;
pub use self::postgres::Postgres;
pub use self::sqlite::Sqlite;

use crate::ast::*;
use std::{borrow::Cow, fmt};

/// A function travelling through the query AST, building the final query string
/// and gathering parameters sent to the database together with the query.
pub trait Visitor<'a> {
    /// Backtick character to surround identifiers, such as column and table names.
    const C_BACKTICK: &'static str;
    /// Wildcard character to be used in `LIKE` queries.
    const C_WILDCARD: &'static str;

    /// Convert the given `Query` to an SQL string and a vector of parameters.
    /// When certain parameters are replaced with the `C_PARAM` character in the
    /// query, the vector should contain the parameter value in the right position.
    ///
    /// The point of entry for visiting query ASTs.
    ///
    /// ```
    /// # use quaint::{ast::*, visitor::*};
    /// let query = Select::from_table("cats");
    /// let (sqlite, _) = Sqlite::build(query.clone());
    /// let (psql, _) = Postgres::build(query.clone());
    /// let (mysql, _) = Mysql::build(query.clone());
    ///
    /// assert_eq!("SELECT `cats`.* FROM `cats`", sqlite);
    /// assert_eq!("SELECT \"cats\".* FROM \"cats\"", psql);
    /// assert_eq!("SELECT `cats`.* FROM `cats`", mysql);
    /// ```
    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue<'a>>)
    where
        Q: Into<Query<'a>>;

    /// Write to the query.
    fn write<D: fmt::Display>(&mut self, s: D) -> fmt::Result;

    fn surround_with<F>(&mut self, begin: &str, end: &str, f: F) -> fmt::Result
    where
        F: FnOnce(&mut Self) -> fmt::Result,
    {
        self.write(begin)?;
        f(self)?;
        self.write(end)
    }

    /// When called, the visitor decided to not render the parameter into the query,
    /// replacing it with the `C_PARAM`, calling `add_parameter` with the replaced value.
    fn add_parameter(&mut self, value: ParameterizedValue<'a>);

    /// The `LIMIT` and `OFFSET` statement in the query
    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue<'a>>,
        offset: Option<ParameterizedValue<'a>>,
    ) -> fmt::Result;

    /// A walk through an `INSERT` statement
    fn visit_insert(&mut self, insert: Insert<'a>) -> fmt::Result;

    /// What to use to substitute a parameter in the query.
    fn parameter_substitution(&mut self) -> fmt::Result;

    /// What to use to substitute a parameter in the query.
    fn visit_aggregate_to_string(&mut self, value: DatabaseValue<'a>) -> fmt::Result;

    /// A visit to a value we parameterize
    fn visit_parameterized(&mut self, value: ParameterizedValue<'a>) -> fmt::Result {
        self.add_parameter(value);
        self.parameter_substitution()
    }

    /// The join statements in the query
    fn visit_joins(&mut self, joins: Vec<Join<'a>>) -> fmt::Result {
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

    fn visit_join_data(&mut self, data: JoinData<'a>) -> fmt::Result {
        self.visit_table(data.table, true)?;
        self.write(" ON ")?;
        self.visit_conditions(data.conditions)
    }

    /// A walk through a `SELECT` statement
    fn visit_select(&mut self, select: Select<'a>) -> fmt::Result {
        self.write("SELECT ")?;

        if let Some(table) = select.table {
            if select.columns.is_empty() {
                match table.typ {
                    TableType::Query(_) | TableType::Values(_) => match table.alias {
                        Some(ref alias) => {
                            self.surround_with(Self::C_BACKTICK, Self::C_BACKTICK, |ref mut s| s.write(alias))?;
                            self.write(".*")?;
                        }
                        None => self.write("*")?,
                    },
                    TableType::Table(_) => match table.alias.clone() {
                        Some(ref alias) => {
                            self.surround_with(Self::C_BACKTICK, Self::C_BACKTICK, |ref mut s| s.write(alias))?;
                            self.write(".*")?;
                        }
                        None => {
                            self.visit_table(*table.clone(), false)?;
                            self.write(".*")?;
                        }
                    },
                }
            } else {
                self.visit_columns(select.columns)?;
            }

            self.write(" FROM ")?;
            self.visit_table(*table, true)?;

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
    fn visit_update(&mut self, update: Update<'a>) -> fmt::Result {
        self.write("UPDATE ")?;
        self.visit_table(update.table, true)?;

        {
            self.write(" SET ")?;
            let pairs = update.columns.into_iter().zip(update.values.into_iter());
            let len = pairs.len();

            for (i, (key, value)) in pairs.enumerate() {
                self.visit_column(key)?;
                self.write(" = ")?;
                self.visit_database_value(value)?;

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
    fn visit_delete(&mut self, delete: Delete<'a>) -> fmt::Result {
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
    fn delimited_identifiers(&mut self, parts: &[&str]) -> fmt::Result {
        let len = parts.len();

        for (i, parts) in parts.iter().enumerate() {
            self.surround_with(Self::C_BACKTICK, Self::C_BACKTICK, |ref mut s| s.write(parts))?;

            if i < (len - 1) {
                self.write(".")?;
            }
        }

        Ok(())
    }

    /// A walk through a complete `Query` statement
    fn visit_query(&mut self, query: Query<'a>) {
        match query {
            Query::Select(select) => self.visit_select(*select).unwrap(),
            Query::Insert(insert) => self.visit_insert(*insert).unwrap(),
            Query::Update(update) => self.visit_update(*update).unwrap(),
            Query::Delete(delete) => self.visit_delete(*delete).unwrap(),
            Query::Union(union) => self.visit_union(union).unwrap(),
            Query::Raw(string) => self.write(string).unwrap(),
        }
    }

    /// A walk through a union of `SELECT` statements
    fn visit_union(&mut self, mut ua: Union<'a>) -> fmt::Result {
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
    fn visit_columns(&mut self, columns: Vec<DatabaseValue<'a>>) -> fmt::Result {
        let len = columns.len();

        for (i, column) in columns.into_iter().enumerate() {
            self.visit_database_value(column)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_operation(&mut self, op: SqlOp<'a>) -> fmt::Result {
        match op {
            SqlOp::Add(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_database_value(left)?;
                se.write(" + ")?;
                se.visit_database_value(right)
            }),
            SqlOp::Sub(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_database_value(left)?;
                se.write(" - ")?;
                se.visit_database_value(right)
            }),
            SqlOp::Mul(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_database_value(left)?;
                se.write(" * ")?;
                se.visit_database_value(right)
            }),
            SqlOp::Div(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_database_value(left)?;
                se.write(" / ")?;
                se.visit_database_value(right)
            }),
            SqlOp::Rem(left, right) => self.surround_with("(", ")", |ref mut se| {
                se.visit_database_value(left)?;
                se.write(" % ")?;
                se.visit_database_value(right)
            }),
        }
    }

    /// A visit to a value used in an expression
    fn visit_database_value(&mut self, value: DatabaseValue<'a>) -> fmt::Result {
        match value {
            DatabaseValue::Parameterized(val) => self.visit_parameterized(val),
            DatabaseValue::Column(column) => self.visit_column(*column),
            DatabaseValue::Row(row) => self.visit_row(row),
            DatabaseValue::Select(select) => self.surround_with("(", ")", |ref mut s| s.visit_select(*select)),
            DatabaseValue::Function(function) => self.visit_function(function),
            DatabaseValue::Op(op) => self.visit_operation(*op),
            DatabaseValue::Values(values) => self.visit_values(*values),
            DatabaseValue::Asterisk(table) => match table {
                Some(table) => {
                    self.visit_table(*table, false)?;
                    self.write(".*")
                }
                None => self.write("*"),
            },
        }
    }

    fn visit_values(&mut self, values: Values<'a>) -> fmt::Result {
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
    fn visit_table(&mut self, table: Table<'a>, include_alias: bool) -> fmt::Result {
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
    fn visit_column(&mut self, column: Column<'a>) -> fmt::Result {
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
    fn visit_row(&mut self, row: Row<'a>) -> fmt::Result {
        self.surround_with("(", ")", |ref mut s| {
            let len = row.values.len();
            for (i, value) in row.values.into_iter().enumerate() {
                s.visit_database_value(value)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }

            Ok(())
        })
    }

    /// A walk through the query conditions
    fn visit_conditions(&mut self, tree: ConditionTree<'a>) -> fmt::Result {
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

    /// An expression that can either be a single value, a set of conditions or
    /// a comparison call
    fn visit_expression(&mut self, expression: Expression<'a>) -> fmt::Result {
        match expression {
            Expression::Value(value) => self.visit_database_value(*value),
            Expression::ConditionTree(tree) => self.visit_conditions(tree),
            Expression::Compare(compare) => self.visit_compare(compare),
        }
    }

    /// A comparison expression
    fn visit_compare(&mut self, compare: Compare<'a>) -> fmt::Result {
        match compare {
            Compare::Equals(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" = ")?;
                self.visit_database_value(*right)
            }
            Compare::NotEquals(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" <> ")?;
                self.visit_database_value(*right)
            }
            Compare::LessThan(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" < ")?;
                self.visit_database_value(*right)
            }
            Compare::LessThanOrEquals(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" <= ")?;
                self.visit_database_value(*right)
            }
            Compare::GreaterThan(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" > ")?;
                self.visit_database_value(*right)
            }
            Compare::GreaterThanOrEquals(left, right) => {
                self.visit_database_value(*left)?;
                self.write(" >= ")?;
                self.visit_database_value(*right)
            }
            Compare::In(left, right) => match (*left, *right) {
                (_, DatabaseValue::Row(ref row)) if row.is_empty() => self.write("1=0"),
                (DatabaseValue::Row(_), DatabaseValue::Values(ref vals)) if vals.row_len() == 0 => self.write("1=0"),
                (DatabaseValue::Row(mut cols), DatabaseValue::Values(vals))
                    if cols.len() == 1 && vals.row_len() == 1 =>
                {
                    let col = cols.pop().unwrap();
                    let vals = vals.flatten_row().unwrap();

                    self.visit_database_value(col)?;
                    self.write(" IN ")?;
                    self.visit_row(vals)
                }
                (left, DatabaseValue::Parameterized(pv)) => {
                    self.visit_database_value(left)?;
                    self.write(" = ")?;
                    self.visit_parameterized(pv)
                }
                (left, dbv) => {
                    self.visit_database_value(left)?;
                    self.write(" IN ")?;
                    self.visit_database_value(dbv)
                }
            },
            Compare::NotIn(left, right) => match (*left, *right) {
                (_, DatabaseValue::Row(ref row)) if row.is_empty() => self.write("1=1"),
                (DatabaseValue::Row(_), DatabaseValue::Values(ref vals)) if vals.row_len() == 0 => self.write("1=1"),
                (DatabaseValue::Row(mut cols), DatabaseValue::Values(vals))
                    if cols.len() == 1 && vals.row_len() == 1 =>
                {
                    let col = cols.pop().unwrap();
                    let vals = vals.flatten_row().unwrap();

                    self.visit_database_value(col)?;
                    self.write(" NOT IN ")?;
                    self.visit_row(vals)
                }
                (left, DatabaseValue::Parameterized(pv)) => {
                    self.visit_database_value(left)?;
                    self.write(" <> ")?;
                    self.visit_parameterized(pv)
                }
                (left, dbv) => {
                    self.visit_database_value(left)?;
                    self.write(" NOT IN ")?;
                    self.visit_database_value(dbv)
                }
            },
            Compare::Like(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                ))));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotLike(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                ))));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::BeginsWith(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    right,
                    Self::C_WILDCARD
                ))));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotBeginsWith(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    right,
                    Self::C_WILDCARD
                ))));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::EndsInto(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    Self::C_WILDCARD,
                    right,
                ))));

                self.write(" LIKE ")?;
                self.parameter_substitution()
            }
            Compare::NotEndsInto(left, right) => {
                self.visit_database_value(*left)?;

                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    Self::C_WILDCARD,
                    right,
                ))));

                self.write(" NOT LIKE ")?;
                self.parameter_substitution()
            }
            Compare::Null(column) => {
                self.visit_database_value(*column)?;
                self.write(" IS NULL")
            }
            Compare::NotNull(column) => {
                self.visit_database_value(*column)?;
                self.write(" IS NOT NULL")
            }
            Compare::Between(val, left, right) => {
                self.visit_database_value(*val)?;
                self.write(" BETWEEN ")?;
                self.visit_database_value(*left)?;
                self.write(" AND ")?;
                self.visit_database_value(*right)
            }
            Compare::NotBetween(val, left, right) => {
                self.visit_database_value(*val)?;
                self.write(" NOT BETWEEN ")?;
                self.visit_database_value(*left)?;
                self.write(" AND ")?;
                self.visit_database_value(*right)
            }
        }
    }

    /// A visit in the `ORDER BY` section of the query
    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> fmt::Result {
        let len = ordering.0.len();

        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            let direction = ordering.map(|dir| match dir {
                Order::Asc => " ASC",
                Order::Desc => " DESC",
            });

            self.visit_database_value(value)?;
            self.write(direction.unwrap_or(""))?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    /// A visit in the `GROUP BY` section of the query
    fn visit_grouping(&mut self, grouping: Grouping<'a>) -> fmt::Result {
        let len = grouping.0.len();

        for (i, value) in grouping.0.into_iter().enumerate() {
            self.visit_database_value(value)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_function(&mut self, fun: Function<'a>) -> fmt::Result {
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
                self.surround_with("(", ")", |ref mut s| s.visit_column(sum.column))?;
            }
        };

        if let Some(alias) = fun.alias {
            self.write(" AS ")?;
            self.delimited_identifiers(&[&*alias])?;
        }

        Ok(())
    }

    fn visit_partitioning(&mut self, over: Over<'a>) -> fmt::Result {
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
