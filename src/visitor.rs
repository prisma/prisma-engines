//! Visitors for reading an abstract SQL syntax tree, generating the query and
//! gathering parameters in the right order.
//!
//! The visitor module should not know how to construct an AST, just how to read
//! one. Everything related to the tree generation is in the
//! [ast](../ast/index.html) module.
//!
//! For prelude, all important imports are in `prisma_query::visitor::*`;
use crate::ast::*;
use std::borrow::Cow;

#[cfg(feature = "rusqlite-0_19")]
mod sqlite;

#[cfg(feature = "rusqlite-0_19")]
pub use self::sqlite::Sqlite;

#[cfg(feature = "postgresql-0_16")]
mod postgres;

#[cfg(feature = "postgresql-0_16")]
pub use self::postgres::Postgres;

#[cfg(feature = "mysql-16")]
mod mysql;

#[cfg(feature = "mysql-16")]
pub use self::mysql::Mysql;

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
    /// use prisma_query::{ast::*, visitor::*};
    ///
    /// fn main() {
    ///     let query = Select::from_table("cats");
    ///     let (sqlite, _) = Sqlite::build(query.clone());
    ///     let (psql, _) = Postgres::build(query.clone());
    ///     let (mysql, _) = Mysql::build(query.clone());
    ///
    ///     assert_eq!("SELECT `cats`.* FROM `cats`", sqlite);
    ///     assert_eq!("SELECT \"cats\".* FROM \"cats\"", psql);
    ///     assert_eq!("SELECT `cats`.* FROM `cats`", mysql);
    /// }
    /// ```
    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue<'a>>)
    where
        Q: Into<Query<'a>>;

    /// When called, the visitor decided to not render the parameter into the query,
    /// replacing it with the `C_PARAM`, calling `add_parameter` with the replaced value.
    fn add_parameter(&mut self, value: ParameterizedValue<'a>);

    /// The `LIMIT` and `OFFSET` statement in the query
    fn visit_limit_and_offset(
        &mut self,
        limit: Option<ParameterizedValue<'a>>,
        offset: Option<ParameterizedValue<'a>>,
    ) -> Option<String>;

    /// A walk through an `INSERT` statement
    fn visit_insert(&mut self, insert: Insert<'a>) -> String;

    /// What to use to substitute a parameter in the query.
    fn parameter_substitution(&self) -> String;

    /// What to use to substitute a parameter in the query.
    fn visit_aggregate_to_string(&mut self, value: DatabaseValue<'a>) -> String;

    /// A visit to a value we parameterize
    fn visit_parameterized(&mut self, value: ParameterizedValue<'a>) -> String {
        self.add_parameter(value);
        self.parameter_substitution()
    }

    /// The join statements in the query
    fn visit_joins(&mut self, joins: Vec<Join<'a>>) -> String {
        let result = joins.into_iter().fold(Vec::new(), |mut acc, j| {
            match j {
                Join::Inner(data) => acc.push(format!("INNER JOIN {}", self.visit_join_data(data))),
                Join::LeftOuter(data) => {
                    acc.push(format!("LEFT OUTER JOIN {}", self.visit_join_data(data)))
                }
            }

            acc
        });

        result.join(" ")
    }

    fn visit_join_data(&mut self, data: JoinData<'a>) -> String {
        format!(
            "{} ON {}",
            self.visit_table(data.table, true),
            self.visit_conditions(data.conditions)
        )
    }

    /// A walk through a `SELECT` statement
    fn visit_select(&mut self, select: Select<'a>) -> String {
        let mut result = vec!["SELECT".to_string()];

        if let Some(table) = select.table {
            if select.columns.is_empty() {
                match table.typ {
                    TableType::Query(_) => match table.alias {
                        Some(ref alias) => {
                            result.push(format!("{}.*", Self::delimited_identifiers(vec![alias])))
                        }
                        None => result.push(String::from("*")),
                    },
                    TableType::Table(_) => {
                        match table.alias.clone() {
                            Some(ref alias) => result
                                .push(format!("{}.*", Self::delimited_identifiers(vec![alias]))),
                            None => result
                                .push(format!("{}.*", self.visit_table(*table.clone(), false))),
                        }
                    }
                }
            } else {
                result.push(self.visit_columns(select.columns));
            }

            result.push(format!("FROM {}", self.visit_table(*table, true)));

            if !select.joins.is_empty() {
                result.push(self.visit_joins(select.joins));
            }

            if let Some(conditions) = select.conditions {
                result.push(format!("WHERE {}", self.visit_conditions(conditions)));
            }
            if !select.ordering.is_empty() {
                result.push(format!("ORDER BY {}", self.visit_ordering(select.ordering)));
            }
            if !select.grouping.is_empty() {
                result.push(format!("GROUP BY {}", self.visit_grouping(select.grouping)));
            }

            if let Some(window) = self.visit_limit_and_offset(select.limit, select.offset) {
                result.push(window);
            }
        } else if select.columns.is_empty() {
            result.push(String::from("*"));
        } else {
            result.push(self.visit_columns(select.columns));
        }

        result.join(" ")
    }

    /// A walk through an `UPDATE` statement
    fn visit_update(&mut self, update: Update<'a>) -> String {
        let mut result = vec![format!(
            "UPDATE {} SET",
            self.visit_table(update.table, true)
        )];

        {
            let pairs = update.columns.into_iter().zip(update.values.into_iter());

            let assignments: Vec<String> = pairs
                .map(|(key, value)| {
                    format!(
                        "{} = {}",
                        self.visit_column(key),
                        self.visit_database_value(value)
                    )
                })
                .collect();

            result.push(assignments.join(", "));
        }

        if let Some(conditions) = update.conditions {
            result.push(format!("WHERE {}", self.visit_conditions(conditions)));
        }

        result.join(" ")
    }

    /// A walk through an `DELETE` statement
    fn visit_delete(&mut self, delete: Delete<'a>) -> String {
        let mut result = vec![format!(
            "DELETE FROM {}",
            self.visit_table(delete.table, true)
        )];

        if let Some(conditions) = delete.conditions {
            result.push(format!("WHERE {}", self.visit_conditions(conditions)));
        }

        result.join(" ")
    }

    /// A helper for delimiting an identifier, surrounding every part with `C_BACKTICK`
    /// and delimiting the values with a `.`
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// assert_eq!(
    ///     "`a`.`b`",
    ///     Sqlite::delimited_identifiers(vec!["a".into(), "b".into()])
    /// );
    /// ```
    fn delimited_identifiers(parts: Vec<&str>) -> String {
        let mut result = Vec::new();

        for part in parts.into_iter() {
            result.push(format!("{}{}{}", Self::C_BACKTICK, part, Self::C_BACKTICK));
        }

        result.join(".")
    }

    /// A walk through a complete `Query` statement
    fn visit_query(&mut self, query: Query<'a>) -> String {
        match query {
            Query::Select(select) => self.visit_select(select),
            Query::Insert(insert) => self.visit_insert(*insert),
            Query::Update(update) => self.visit_update(*update),
            Query::Delete(delete) => self.visit_delete(*delete),
            Query::UnionAll(union) => self.visit_union_all(union),
            Query::Raw(string) => string.into_owned(),
        }
    }

    /// A walk through a union of `SELECT` statements
    fn visit_union_all(&mut self, ua: UnionAll<'a>) -> String {
        let selects: Vec<String> =
            ua.0.into_iter()
                .map(|s| format!("({})", self.visit_select(s)))
                .collect();

        selects.join(" UNION ALL ")
    }

    /// The selected columns
    fn visit_columns(&mut self, columns: Vec<DatabaseValue<'a>>) -> String {
        let mut values = Vec::new();

        for column in columns.into_iter() {
            values.push(self.visit_database_value(column));
        }

        values.join(", ")
    }

    /// A visit to a value used in an expression
    fn visit_database_value(&mut self, value: DatabaseValue<'a>) -> String {
        match value {
            DatabaseValue::Parameterized(val) => self.visit_parameterized(val),
            DatabaseValue::Column(column) => self.visit_column(*column),
            DatabaseValue::Row(row) => self.visit_row(row),
            DatabaseValue::Select(select) => format!("({})", self.visit_select(select)),
            DatabaseValue::Function(function) => self.visit_function(function),
            DatabaseValue::Asterisk(table) => match table {
                Some(table) => format!("{}.*", self.visit_table(*table, false)),
                None => String::from("*"),
            },
        }
    }

    /// A database table identifier
    fn visit_table(&mut self, table: Table<'a>, include_alias: bool) -> String {
        let mut result = match table.typ {
            TableType::Table(table_name) => match table.database {
                Some(database) => Self::delimited_identifiers(vec![&*database, &*table_name]),
                None => Self::delimited_identifiers(vec![&*table_name]),
            },
            TableType::Query(select) => format!("({})", self.visit_select(select)),
        };

        if include_alias {
            if let Some(alias) = table.alias {
                result.push_str(" AS ");
                result.push_str(&Self::delimited_identifiers(vec![&*alias]));
            };
        }

        result
    }

    /// A database column identifier
    fn visit_column(&mut self, column: Column<'a>) -> String {
        let mut column_identifier = match column.table {
            Some(table) => format!(
                "{}.{}",
                self.visit_table(table, false),
                Self::delimited_identifiers(vec![&*column.name])
            ),
            _ => Self::delimited_identifiers(vec![&*column.name]),
        };

        if let Some(alias) = column.alias {
            column_identifier.push_str(" AS ");
            column_identifier.push_str(&Self::delimited_identifiers(vec![&*alias]));
        }

        column_identifier
    }

    /// A row of data used as an expression
    fn visit_row(&mut self, row: Row<'a>) -> String {
        let mut values = Vec::new();

        for value in row.values.into_iter() {
            values.push(self.visit_database_value(value));
        }

        format!("({})", values.join(", "))
    }

    /// A walk through the query conditions
    fn visit_conditions(&mut self, tree: ConditionTree<'a>) -> String {
        match tree {
            ConditionTree::And(left, right) => format!(
                "({} AND {})",
                self.visit_expression(*left),
                self.visit_expression(*right),
            ),
            ConditionTree::Or(left, right) => format!(
                "({} OR {})",
                self.visit_expression(*left),
                self.visit_expression(*right),
            ),
            ConditionTree::Not(expression) => {
                format!("(NOT {})", self.visit_expression(*expression))
            }
            ConditionTree::Single(expression) => self.visit_expression(*expression),
            ConditionTree::NoCondition => String::from("1=1"),
            ConditionTree::NegativeCondition => String::from("1=0"),
        }
    }

    /// An expression that can either be a single value, a set of conditions or
    /// a comparison call
    fn visit_expression(&mut self, expression: Expression<'a>) -> String {
        match expression {
            Expression::Value(value) => self.visit_database_value(*value),
            Expression::ConditionTree(tree) => self.visit_conditions(tree),
            Expression::Compare(compare) => self.visit_compare(compare),
        }
    }

    /// A comparison expression
    fn visit_compare(&mut self, compare: Compare<'a>) -> String {
        match compare {
            Compare::Equals(left, right) => format!(
                "{} = {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::NotEquals(left, right) => format!(
                "{} <> {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::LessThan(left, right) => format!(
                "{} < {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::LessThanOrEquals(left, right) => format!(
                "{} <= {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::GreaterThan(left, right) => format!(
                "{} > {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::GreaterThanOrEquals(left, right) => format!(
                "{} >= {}",
                self.visit_database_value(*left),
                self.visit_database_value(*right),
            ),
            Compare::In(left, right) => match *right {
                DatabaseValue::Row(ref row) if row.is_empty() => String::from("1=0"),
                _ => format!(
                    "{} IN {}",
                    self.visit_database_value(*left),
                    self.visit_database_value(*right),
                ),
            },
            Compare::NotIn(left, right) => match *right {
                DatabaseValue::Row(ref row) if row.is_empty() => String::from("1=1"),
                _ => format!(
                    "{} NOT IN {}",
                    self.visit_database_value(*left),
                    self.visit_database_value(*right),
                ),
            },
            Compare::Like(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                ))));
                format!("{} LIKE {}", expression, self.parameter_substitution())
            }
            Compare::NotLike(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}{}",
                    Self::C_WILDCARD,
                    right,
                    Self::C_WILDCARD
                ))));
                format!("{} NOT LIKE {}", expression, self.parameter_substitution())
            }
            Compare::BeginsWith(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    right,
                    Self::C_WILDCARD
                ))));
                format!("{} LIKE {}", expression, self.parameter_substitution())
            }
            Compare::NotBeginsWith(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    right,
                    Self::C_WILDCARD
                ))));
                format!("{} NOT LIKE {}", expression, self.parameter_substitution())
            }
            Compare::EndsInto(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    Self::C_WILDCARD,
                    right
                ))));
                format!("{} LIKE {}", expression, self.parameter_substitution())
            }
            Compare::NotEndsInto(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(Cow::from(format!(
                    "{}{}",
                    Self::C_WILDCARD,
                    right
                ))));
                format!("{} NOT LIKE {}", expression, self.parameter_substitution())
            }
            Compare::Null(column) => format!("{} IS NULL", self.visit_database_value(*column)),
            Compare::NotNull(column) => {
                format!("{} IS NOT NULL", self.visit_database_value(*column))
            }
            Compare::Between(val, left, right) => format!(
                "{} BETWEEN {} AND {}",
                self.visit_database_value(*val),
                self.visit_database_value(*left),
                self.visit_database_value(*right)
            ),
            Compare::NotBetween(val, left, right) => format!(
                "{} NOT BETWEEN {} AND {}",
                self.visit_database_value(*val),
                self.visit_database_value(*left),
                self.visit_database_value(*right)
            ),
        }
    }

    /// A visit in the `ORDER BY` section of the query
    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> String {
        let mut result = Vec::new();

        for (value, ordering) in ordering.0.into_iter() {
            let direction = ordering.map(|dir| match dir {
                Order::Asc => " ASC",
                Order::Desc => " DESC",
            });

            result.push(format!(
                "{}{}",
                self.visit_database_value(value),
                direction.unwrap_or("")
            ));
        }

        result.join(", ")
    }

    /// A visit in the `GROUP BY` section of the query
    fn visit_grouping(&mut self, grouping: Grouping<'a>) -> String {
        let mut result = Vec::new();

        for value in grouping.0.into_iter() {
            result.push(self.visit_database_value(value).to_string());
        }

        result.join(", ")
    }

    fn visit_function(&mut self, fun: Function<'a>) -> String {
        let mut result = match fun.typ_ {
            FunctionType::RowNumber(fun_rownum) => {
                if fun_rownum.over.is_empty() {
                    String::from("ROW_NUMBER() OVER()")
                } else {
                    format!(
                        "ROW_NUMBER() OVER({})",
                        self.visit_partitioning(fun_rownum.over)
                    )
                }
            }
            FunctionType::Count(fun_count) => {
                if fun_count.exprs.is_empty() {
                    String::from("COUNT(*)")
                } else {
                    format!("COUNT({})", self.visit_columns(fun_count.exprs))
                }
            }
            FunctionType::AggregateToString(agg) => {
                self.visit_aggregate_to_string(agg.value.as_ref().clone())
            }
        };

        if let Some(alias) = fun.alias {
            result.push_str(" AS ");
            result.push_str(&Self::delimited_identifiers(vec![&*alias]));
        }

        result
    }

    fn visit_partitioning(&mut self, over: Over<'a>) -> String {
        let mut result = Vec::new();

        if !over.partitioning.is_empty() {
            let mut parts = Vec::new();

            for partition in over.partitioning {
                parts.push(self.visit_column(partition))
            }

            result.push(format!("PARTITION BY {}", parts.join(", ")));
        }

        if !over.ordering.is_empty() {
            result.push(format!("ORDER BY {}", self.visit_ordering(over.ordering)));
        }

        result.join(" ")
    }
}
