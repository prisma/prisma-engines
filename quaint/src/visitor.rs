//! Visitors for reading an abstract SQL syntax tree, generating the query and
//! gathering parameters in the right order.
//!
//! The visitor module should not know how to construct an AST, just how to read
//! one. Everything related to the tree generation is in the
//! [ast](../ast/index.html) module.
//!
//! For prelude, all important imports are in `quaint::visitor::*`;
#[cfg(feature = "mssql")]
mod mssql;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgresql")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

// Generic query writer, used for all SQL flavors
mod query_writer;

#[cfg(feature = "mssql")]
pub use self::mssql::Mssql;
#[cfg(feature = "mysql")]
pub use self::mysql::Mysql;
#[cfg(feature = "postgresql")]
pub use self::postgres::Postgres;
#[cfg(feature = "sqlite")]
pub use self::sqlite::Sqlite;

use crate::ast::*;
use query_template::QueryTemplate;
use std::{borrow::Cow, fmt};

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
        Q: Into<Query<'a>>,
    {
        let template = Self::build_template(query)?;
        let sql = template.to_sql()?;
        Ok((sql, template.parameters))
    }

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>;

    /// Write to the query.
    fn write(&mut self, s: impl fmt::Display) -> Result;

    /// A point to modify an incoming query to make it compatible with the
    /// underlying database.
    fn compatibility_modifications(&self, query: Query<'a>) -> Query<'a> {
        query
    }

    fn surround_with<F>(&mut self, begin: &str, end: &str, f: F) -> Result
    where
        F: FnOnce(&mut Self) -> Result,
    {
        self.write(begin)?;
        f(self)?;
        self.write(end)
    }

    fn columns_to_bracket_list(&mut self, columns: Vec<Column<'a>>) -> Result {
        let len = columns.len();

        self.write(" (")?;
        for (i, c) in columns.into_iter().enumerate() {
            self.visit_column(c.name.into_owned().into())?;

            if i < (len - 1) {
                self.write(",")?;
            }
        }
        self.write(")")?;

        Ok(())
    }

    /// When called, the visitor decided to not render the parameter into the query,
    /// replacing it with the `C_PARAM`, calling `add_parameter` with the replaced value.
    fn add_parameter(&mut self, value: Value<'a>);

    /// The `LIMIT` and `OFFSET` statement in the query
    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> Result;

    /// A visit in the `ORDER BY` section of the query
    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> Result;

    /// A walk through an `INSERT` statement
    fn visit_insert(&mut self, insert: Insert<'a>) -> Result;

    /// What to use to substitute a parameter in the query.
    fn parameter_substitution(&mut self) -> Result;

    /// What to use to substitute a list of parameters of variable length
    fn visit_parameterized_row(
        &mut self,
        value: Value<'a>,
        item_prefix: impl Into<Cow<'static, str>>,
        separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
    ) -> Result;

    /// What to use to aggregate an array of values into a string
    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> Result;

    /// Visit a non-parameterized value.
    fn visit_raw_value(&mut self, value: Value<'a>) -> Result;

    // TODO: JSON functions such as this one should only be required when
    // `#[cfg(any(feature = "postgresql", feature = "mysql"))]` or similar filters apply.
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> Result;

    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> Result;

    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> Result;

    fn visit_json_array_contains(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> Result;

    fn visit_json_type_equals(&mut self, left: Expression<'a>, right: JsonType<'a>, not: bool) -> Result;

    fn visit_json_unquote(&mut self, json_unquote: JsonUnquote<'a>) -> Result;

    fn visit_json_array_agg(&mut self, array_agg: JsonArrayAgg<'a>) -> Result;

    fn visit_json_build_object(&mut self, build_obj: JsonBuildObject<'a>) -> Result;

    fn visit_text_search(&mut self, text_search: TextSearch<'a>) -> Result;

    fn visit_matches(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> Result;

    fn visit_text_search_relevance(&mut self, text_search_relevance: TextSearchRelevance<'a>) -> Result;

    fn visit_parameterized_enum(&mut self, variant: EnumVariant<'a>, name: Option<EnumName<'a>>) -> Result {
        match name {
            Some(name) => self.add_parameter(Value::enum_variant_with_name(variant, name)),
            None => self.add_parameter(Value::enum_variant(variant)),
        }
        self.parameter_substitution()?;

        Ok(())
    }

    fn visit_parameterized_enum_array(&mut self, variants: Vec<EnumVariant<'a>>, name: Option<EnumName<'a>>) -> Result {
        let enum_variants: Vec<_> = variants
            .into_iter()
            .map(|variant| variant.into_enum(name.clone()))
            .collect();

        self.add_parameter(Value::array(enum_variants));
        self.parameter_substitution()?;

        Ok(())
    }

    fn visit_parameterized_text(&mut self, txt: Option<Cow<'a, str>>, nt: Option<NativeColumnType<'a>>) -> Result {
        self.add_parameter(Value {
            typed: ValueType::Text(txt),
            native_column_type: nt,
        });
        self.parameter_substitution()?;

        Ok(())
    }

    /// A visit to a value we parameterize
    fn visit_parameterized(&mut self, value: Value<'a>) -> Result {
        match value.typed {
            ValueType::Enum(Some(variant), name) => self.visit_parameterized_enum(variant, name),
            ValueType::EnumArray(Some(variants), name) => self.visit_parameterized_enum_array(variants, name),
            ValueType::Text(txt) => self.visit_parameterized_text(txt, value.native_column_type),
            _ => {
                self.add_parameter(value);
                self.parameter_substitution()
            }
        }
    }

    /// The join statements in the query
    fn visit_joins(&mut self, joins: Vec<Join<'a>>) -> Result {
        for j in joins {
            match j {
                Join::Inner(data) => {
                    self.write(" INNER JOIN ")?;

                    if data.lateral {
                        self.write("LATERAL ")?;
                    }

                    self.visit_join_data(data)?;
                }
                Join::Left(data) => {
                    self.write(" LEFT JOIN ")?;

                    if data.lateral {
                        self.write("LATERAL ")?;
                    }

                    self.visit_join_data(data)?;
                }
                Join::Right(data) => {
                    self.write(" RIGHT JOIN ")?;

                    if data.lateral {
                        self.write("LATERAL ")?;
                    }

                    self.visit_join_data(data)?;
                }
                Join::Full(data) => {
                    self.write(" FULL JOIN ")?;

                    if data.lateral {
                        self.write("LATERAL ")?;
                    }

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
        let number_of_ctes = select.ctes.len();

        if number_of_ctes > 0 {
            self.write("WITH ")?;

            for (i, cte) in select.ctes.into_iter().enumerate() {
                self.visit_cte(cte)?;

                if i < (number_of_ctes - 1) {
                    self.write(", ")?;
                }
            }

            self.write(" ")?;
        }

        self.write("SELECT ")?;

        if let Some(distinct) = select.distinct {
            match distinct {
                DistinctType::Default => self.write("DISTINCT ")?,
                DistinctType::OnClause(columns) => {
                    self.write("DISTINCT ON ")?;
                    self.surround_with("(", ") ", |ref mut s| s.visit_columns(columns))?;
                }
            }
        };

        if !select.tables.is_empty() {
            if select.columns.is_empty() {
                for (i, table) in select.tables.iter().enumerate() {
                    if i > 0 {
                        self.write(", ")?;
                    }

                    match &table.typ {
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
                                self.visit_table(table.clone(), false)?;
                                self.write(".*")?;
                            }
                        },
                        TableType::JoinedTable(jt) => match table.alias.clone() {
                            Some(ref alias) => {
                                self.surround_with(Self::C_BACKTICK_OPEN, Self::C_BACKTICK_CLOSE, |ref mut s| {
                                    s.write(alias)
                                })?;
                                self.write(".*")?;
                            }
                            None => {
                                let mut unjoined_table = table.clone();
                                // Convert the table typ to a `TableType::Table` for the SELECT statement print
                                // We only want the join to appear in the FROM clause
                                unjoined_table.typ = TableType::Table(jt.0.clone());

                                self.visit_table(unjoined_table, false)?;
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

                self.visit_table(table, true)?;
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

        if let Some(comment) = select.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    /// A walk through an `UPDATE` statement
    fn visit_update(&mut self, update: Update<'a>) -> Result {
        self.write("UPDATE ")?;
        self.visit_table(update.table, true)?;

        {
            self.write(" SET ")?;
            let pairs = update.columns.into_iter().zip(update.values);
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

        if let Some(returning) = update.returning
            && !returning.is_empty()
        {
            let values = returning.into_iter().map(|r| r.into()).collect();
            self.write(" RETURNING ")?;
            self.visit_columns(values)?;
        }

        if let Some(comment) = update.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    fn visit_upsert(&mut self, update: Update<'a>) -> Result {
        self.write("UPDATE ")?;

        self.write("SET ")?;
        self.visit_update_set(update.clone())?;

        if let Some(conditions) = update.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        Ok(())
    }

    fn visit_update_set(&mut self, update: Update<'a>) -> Result {
        let pairs = update.columns.into_iter().zip(update.values);
        let len = pairs.len();

        for (i, (key, value)) in pairs.enumerate() {
            self.visit_column(key)?;
            self.write(" = ")?;
            self.visit_expression(value)?;

            if i < (len - 1) {
                self.write(", ")?;
            }
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

        if let Some(comment) = delete.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    /// A helper for delimiting an identifier, surrounding every part with `C_BACKTICK`
    /// and delimiting the values with a `.`
    fn delimited_identifiers(&mut self, parts: &[&str]) -> Result {
        let len = parts.len();

        for (i, part) in parts.iter().enumerate() {
            self.surround_with_backticks(part)?;

            if i < (len - 1) {
                self.write(".")?;
            }
        }

        Ok(())
    }

    /// A helper for delimiting a part of an identifier, surrounding it with `C_BACKTICK`
    fn surround_with_backticks(&mut self, part: &str) -> Result {
        self.surround_with(Self::C_BACKTICK_OPEN, Self::C_BACKTICK_CLOSE, |ref mut s| s.write(part))?;
        Ok(())
    }

    /// Visit an SQL `MERGE` query.
    fn visit_merge(&mut self, _merge: Merge<'a>) -> Result {
        unimplemented!("Merges not supported for the underlying database.")
    }

    /// A walk through a complete `Query` statement
    fn visit_query(&mut self, mut query: Query<'a>) -> Result {
        query = self.compatibility_modifications(query);

        match query {
            Query::Select(select) => self.visit_select(*select),
            Query::Insert(insert) => self.visit_insert(*insert),
            Query::Update(update) => self.visit_update(*update),
            Query::Delete(delete) => self.visit_delete(*delete),
            Query::Union(union) => self.visit_union(*union),
            Query::Merge(merge) => self.visit_merge(*merge),
            Query::Raw(string) => self.write(string),
        }
    }

    fn visit_sub_selection(&mut self, query: SelectQuery<'a>) -> Result {
        self.visit_selection(query)
    }

    fn visit_selection(&mut self, query: SelectQuery<'a>) -> Result {
        match query {
            SelectQuery::Select(select) => self.visit_select(*select),
            SelectQuery::Union(union) => self.visit_union(*union),
        }
    }

    /// A walk through a union of `SELECT` statements
    fn visit_union(&mut self, mut ua: Union<'a>) -> Result {
        let number_of_ctes = ua.ctes.len();

        if number_of_ctes > 0 {
            self.write("WITH ")?;

            for (i, cte) in ua.ctes.into_iter().enumerate() {
                self.visit_cte(cte)?;

                if i < (number_of_ctes - 1) {
                    self.write(", ")?;
                }
            }

            self.write(" ")?;
        }

        let len = ua.selects.len();
        let mut types = ua.types.drain(0..);

        for (i, sel) in ua.selects.into_iter().enumerate() {
            self.visit_select(sel)?;

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
            ExpressionKind::ParameterizedRow(val) => self.visit_parameterized_row(val, "", ",", "")?,
            ExpressionKind::RawValue(val) => self.visit_raw_value(val.0)?,
            ExpressionKind::Column(column) => self.visit_column(*column)?,
            ExpressionKind::Row(row) => self.visit_row(row)?,
            ExpressionKind::Selection(selection) => {
                self.surround_with("(", ")", |ref mut s| s.visit_sub_selection(selection))?
            }
            ExpressionKind::Function(function) => self.visit_function(*function)?,
            ExpressionKind::Op(op) => self.visit_operation(*op)?,
            ExpressionKind::Values(values) => self.visit_values(*values)?,
            ExpressionKind::Asterisk(table) => match table {
                Some(table) => {
                    self.visit_table(*table, false)?;
                    self.write(".*")?
                }
                None => self.write("*")?,
            },
            ExpressionKind::Default => self.write("DEFAULT")?,
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
            TableType::Query(select) => self.surround_with("(", ")", |ref mut s| s.visit_select(*select))?,
            TableType::JoinedTable(jt) => {
                match table.database {
                    Some(database) => self.delimited_identifiers(&[&*database, &*jt.0])?,
                    None => self.delimited_identifiers(&[&*jt.0])?,
                }
                self.visit_joins(jt.1)?
            }
        };

        if include_alias && let Some(alias) = table.alias {
            self.write(" AS ")?;

            self.delimited_identifiers(&[&*alias])?;
        };

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

    fn visit_greater_than(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" > ")?;
        self.visit_expression(right)
    }

    fn visit_greater_than_or_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" >= ")?;
        self.visit_expression(right)
    }

    fn visit_less_than(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" < ")?;
        self.visit_expression(right)
    }

    fn visit_less_than_or_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" <= ")?;
        self.visit_expression(right)
    }

    fn visit_like(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" LIKE ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_not_like(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" NOT LIKE ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    /// A comparison expression
    fn visit_compare(&mut self, compare: Compare<'a>) -> Result {
        match compare {
            Compare::Equals(left, right) => self.visit_equals(*left, *right),
            Compare::NotEquals(left, right) => self.visit_not_equals(*left, *right),
            Compare::LessThan(left, right) => self.visit_less_than(*left, *right),
            Compare::LessThanOrEquals(left, right) => self.visit_less_than_or_equals(*left, *right),
            Compare::GreaterThan(left, right) => self.visit_greater_than(*left, *right),
            Compare::GreaterThanOrEquals(left, right) => self.visit_greater_than_or_equals(*left, *right),
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

                // Flattening out a parameterized row with a single column.
                (
                    Expression {
                        kind: ExpressionKind::Row(mut cols),
                        ..
                    },
                    rhs @ Expression {
                        kind: ExpressionKind::ParameterizedRow(_),
                        ..
                    },
                ) if cols.len() == 1 => {
                    let col = cols.pop().unwrap();
                    self.visit_compare(Compare::In(Box::new(col), Box::new(rhs)))
                }

                // expr IN (?, ?, ..., ?)
                (
                    left,
                    Expression {
                        kind: ExpressionKind::ParameterizedRow(value),
                        ..
                    },
                ) => {
                    self.visit_expression(left)?;
                    self.write(" IN ")?;
                    self.visit_parameterized_row(value, "", ",", "")
                }

                // expr IN (CALL(?), CALL(?), ..., CALL(?))
                (
                    left,
                    Expression {
                        kind: ExpressionKind::Function(value),
                        ..
                    },
                ) if value.typ_.arguments().len() == 1
                    && value
                        .typ_
                        .arguments()
                        .iter()
                        .all(|arg| matches!(arg.kind, ExpressionKind::ParameterizedRow(_))) =>
                {
                    self.visit_expression(left)?;
                    self.write(" IN ")?;

                    let Some(ExpressionKind::ParameterizedRow(val)) =
                        value.typ_.arguments().first().map(|arg| &arg.kind)
                    else {
                        unreachable!()
                    };
                    let Some(function_name) = &value.typ_.name() else {
                        panic!("function call against a row of expressions must have a name")
                    };

                    self.visit_parameterized_row(val.clone(), format!("{function_name}("), ",", ")")
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

                // Flattening out a parameterized row with a single column.
                (
                    Expression {
                        kind: ExpressionKind::Row(mut cols),
                        ..
                    },
                    rhs @ Expression {
                        kind: ExpressionKind::ParameterizedRow(_),
                        ..
                    },
                ) if cols.len() == 1 => {
                    let col = cols.pop().unwrap();
                    self.visit_compare(Compare::NotIn(Box::new(col), Box::new(rhs)))
                }

                // expr NOT IN (?, ?, ..., ?)
                (
                    left,
                    Expression {
                        kind: ExpressionKind::ParameterizedRow(value),
                        ..
                    },
                ) => {
                    self.visit_expression(left)?;
                    self.write(" NOT IN ")?;
                    self.visit_parameterized_row(value, "", ",", "")
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

                // expr NOT IN (CALL(?), CALL(?), ..., CALL(?))
                (
                    left,
                    Expression {
                        kind: ExpressionKind::Function(value),
                        ..
                    },
                ) if value.typ_.arguments().len() == 1
                    && value
                        .typ_
                        .arguments()
                        .iter()
                        .all(|arg| matches!(arg.kind, ExpressionKind::ParameterizedRow(_))) =>
                {
                    self.visit_expression(left)?;
                    self.write(" NOT IN ")?;

                    let Some(ExpressionKind::ParameterizedRow(val)) =
                        value.typ_.arguments().first().map(|arg| &arg.kind)
                    else {
                        unreachable!()
                    };
                    let Some(function_name) = &value.typ_.name() else {
                        panic!("function call against a row of expressions must have a name")
                    };

                    self.visit_parameterized_row(val.clone(), format!("{function_name}("), ",", ")")
                }

                // expr IN (..)
                (left, right) => {
                    self.visit_expression(left)?;
                    self.write(" NOT IN ")?;
                    self.visit_expression(right)
                }
            },
            Compare::Like(left, right) => self.visit_like(*left, *right),
            Compare::NotLike(left, right) => self.visit_not_like(*left, *right),
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
            Compare::Raw(left, comp, right) => {
                self.visit_expression(*left)?;
                self.write(" ")?;
                self.write(comp)?;
                self.write(" ")?;
                self.visit_expression(*right)
            }
            Compare::JsonCompare(json_compare) => match json_compare {
                JsonCompare::ArrayContains(left, right) => self.visit_json_array_contains(*left, *right, false),
                JsonCompare::ArrayNotContains(left, right) => self.visit_json_array_contains(*left, *right, true),
                JsonCompare::TypeEquals(left, json_type) => self.visit_json_type_equals(*left, json_type, false),
                JsonCompare::TypeNotEquals(left, json_type) => self.visit_json_type_equals(*left, json_type, true),
            },
            Compare::Matches(left, right) => self.visit_matches(*left, *right, false),
            Compare::NotMatches(left, right) => self.visit_matches(*left, *right, true),
            Compare::Any(left) => {
                self.write("ANY")?;
                self.surround_with("(", ")", |s| s.visit_expression(*left))
            }
            Compare::All(left) => {
                self.write("ALL")?;
                self.surround_with("(", ")", |s| s.visit_expression(*left))
            }
            Compare::Exists(query) => {
                self.write("EXISTS")?;
                self.surround_with("(", ")", |s| s.visit_sub_selection(*query))
            }
            Compare::NotExists(query) => {
                self.write("NOT EXISTS")?;
                self.surround_with("(", ")", |s| s.visit_sub_selection(*query))
            }
        }
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;

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

    fn visit_average(&mut self, avg: Average<'a>) -> Result {
        self.write("AVG")?;
        self.surround_with("(", ")", |ref mut s| s.visit_column(avg.column))?;
        Ok(())
    }

    fn visit_min(&mut self, min: Minimum<'a>) -> Result {
        self.write("MIN")?;
        self.surround_with("(", ")", |ref mut s| s.visit_column(min.column))?;

        Ok(())
    }

    fn visit_max(&mut self, max: Maximum<'a>) -> Result {
        self.write("MAX")?;
        self.surround_with("(", ")", |ref mut s| s.visit_column(max.column))?;

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
            FunctionType::RowToJson(row_to_json) => {
                self.write("ROW_TO_JSON")?;
                self.surround_with("(", ")", |ref mut s| s.visit_table(row_to_json.expr, false))?
            }
            FunctionType::Average(avg) => {
                self.visit_average(avg)?;
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
                self.visit_min(min)?;
            }
            FunctionType::Maximum(max) => {
                self.visit_max(max)?;
            }
            FunctionType::Coalesce(coalesce) => {
                self.write("COALESCE")?;
                self.surround_with("(", ")", |s| s.visit_columns(coalesce.exprs))?;
            }
            FunctionType::JsonExtract(json_extract) => {
                self.visit_json_extract(json_extract)?;
            }
            FunctionType::JsonExtractFirstArrayElem(extract) => {
                self.visit_json_extract_first_array_item(extract)?;
            }
            FunctionType::JsonExtractLastArrayElem(extract) => {
                self.visit_json_extract_last_array_item(extract)?;
            }
            FunctionType::JsonUnquote(unquote) => {
                self.visit_json_unquote(unquote)?;
            }
            FunctionType::TextSearch(text_search) => {
                self.visit_text_search(text_search)?;
            }
            FunctionType::TextSearchRelevance(text_search_relevance) => {
                self.visit_text_search_relevance(text_search_relevance)?;
            }
            FunctionType::UuidToBin => {
                self.write("uuid_to_bin(uuid())")?;
            }
            FunctionType::UuidToBinSwapped => {
                self.write("uuid_to_bin(uuid(), 1)")?;
            }
            FunctionType::Uuid => self.write("uuid()")?,
            FunctionType::Concat(concat) => {
                self.visit_concat(concat)?;
            }
            FunctionType::JsonArrayAgg(array_agg) => {
                self.visit_json_array_agg(array_agg)?;
            }
            FunctionType::JsonBuildObject(build_obj) => {
                self.visit_json_build_object(build_obj)?;
            }
        };

        if let Some(alias) = fun.alias {
            self.write(" AS ")?;
            self.delimited_identifiers(&[&*alias])?;
        }

        Ok(())
    }

    fn visit_concat(&mut self, concat: Concat<'a>) -> Result {
        let len = concat.exprs.len();

        self.write("CONCAT")?;
        self.surround_with("(", ")", |s| {
            for (i, expr) in concat.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(", ")?;
                }
            }

            Ok(())
        })?;

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

    fn visit_cte(&mut self, cte: CommonTableExpression<'a>) -> Result {
        let cols = cte
            .columns
            .into_iter()
            .map(|s| Column::from(s.into_owned()))
            .collect::<Vec<_>>();

        self.visit_column(Column::from(cte.identifier.into_owned()))?;

        if !cols.is_empty() {
            self.write(" ")?;
            self.visit_row(Row::from(cols))?;
        }

        self.write(" AS ")?;

        let selection = cte.selection;
        self.surround_with("(", ")", |ref mut s| s.visit_selection(selection))
    }

    fn visit_comment(&mut self, comment: Cow<'a, str>) -> Result {
        self.surround_with("/* ", " */", |ref mut s| s.write(comment))
    }
}
