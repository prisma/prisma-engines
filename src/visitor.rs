use crate::ast::*;

#[cfg(feature = "rusqlite")]
mod sqlite;

#[cfg(feature = "rusqlite")]
pub use self::sqlite::Sqlite;

pub trait Visitor {
    const C_PARAM: &'static str;
    const C_QUOTE: &'static str;

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue>)
    where
        Q: Into<Query>;

    fn add_parameter(&mut self, value: ParameterizedValue);
    fn visit_limit(&mut self, limit: Option<usize>) -> String;
    fn visit_offset(&mut self, offset: usize) -> String;

    fn visit_select(&mut self, select: Select) -> String {
        let mut result = vec!["SELECT".to_string()];

        if select.columns.is_empty() {
            result.push(String::from("*"));
        } else {
            result.push(format!("{}", self.visit_columns(select.columns)));
        }

        if let Some(table) = select.table {
            result.push(format!("FROM {}", Self::visit_table(table)));

            if let Some(conditions) = select.conditions {
                result.push(format!("WHERE {}", self.visit_conditions(conditions)));
            }
            if !select.ordering.is_empty() {
                result.push(format!("ORDER BY {}", self.visit_ordering(select.ordering)));
            }

            result.push(self.visit_limit(select.limit));

            if let Some(offset) = select.offset {
                result.push(self.visit_offset(offset));
            }
        }

        result.join(" ")
    }

    fn delimited_identifiers(parts: Vec<String>) -> String {
        let mut result = Vec::new();

        for part in parts.into_iter() {
            result.push(format!("{}{}{}", Self::C_QUOTE, part, Self::C_QUOTE));
        }

        result.join(".")
    }

    fn visit_query(&mut self, query: Query) -> String {
        match query {
            Query::Select(select) => self.visit_select(select),
        }
    }

    fn visit_columns(&mut self, columns: Vec<DatabaseValue>) -> String {
        let mut values = Vec::new();

        for column in columns.into_iter() {
            values.push(self.visit_database_value(column));
        }

        values.join(", ")
    }

    fn visit_database_value(&mut self, value: DatabaseValue) -> String {
        match value {
            DatabaseValue::Parameterized(val) => {
                self.add_parameter(val);
                Self::C_PARAM.to_string()
            }
            DatabaseValue::Column(column) => Self::visit_column(column),
            DatabaseValue::Row(row) => self.visit_row(row),
            DatabaseValue::Select(select) => format!("({})", self.visit_select(select)),
        }
    }

    fn visit_table(table: Table) -> String {
        if let Some(database) = table.database {
            Self::delimited_identifiers(vec![database, table.name])
        } else {
            Self::delimited_identifiers(vec![table.name])
        }
    }

    fn visit_column(column: Column) -> String {
        match column.table {
            Some(table) => format!(
                "{}.{}",
                Self::visit_table(table),
                Self::delimited_identifiers(vec![column.name])
            ),
            _ => Self::delimited_identifiers(vec![column.name]),
        }
    }

    fn visit_row(&mut self, row: Row) -> String {
        let mut values = Vec::new();

        for value in row.values.into_iter() {
            values.push(self.visit_database_value(value));
        }

        format!("({})", values.join(", "))
    }

    fn visit_conditions(&mut self, tree: ConditionTree) -> String {
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

    fn visit_expression(&mut self, expression: Expression) -> String {
        match expression {
            Expression::Value(value) => self.visit_database_value(value),
            Expression::ConditionTree(tree) => self.visit_conditions(tree),
            Expression::Compare(compare) => self.visit_compare(compare),
        }
    }

    fn visit_compare(&mut self, compare: Compare) -> String {
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
            Compare::In(left, right) => format!(
                "{} IN {}",
                self.visit_database_value(*left),
                self.visit_row(*right),
            ),
            Compare::NotIn(left, right) => format!(
                "{} NOT IN {}",
                self.visit_database_value(*left),
                self.visit_row(*right),
            ),
            Compare::Like(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("%{}%", right)));
                format!("{} LIKE ?", expression)
            }
            Compare::NotLike(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("%{}%", right)));
                format!("{} NOT LIKE ?", expression)
            }
            Compare::BeginsWith(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("{}%", right)));
                format!("{} LIKE ?", expression)
            }
            Compare::NotBeginsWith(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("{}%", right)));
                format!("{} NOT LIKE ?", expression)
            }
            Compare::EndsInto(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("%{}", right)));
                format!("{} LIKE ?", expression)
            }
            Compare::NotEndsInto(left, right) => {
                let expression = self.visit_database_value(*left);
                self.add_parameter(ParameterizedValue::Text(format!("%{}", right)));
                format!("{} NOT LIKE ?", expression)
            }
            Compare::Null(column) => format!("{} IS NULL", self.visit_database_value(*column)),
            Compare::NotNull(column) => {
                format!("{} IS NOT NULL", self.visit_database_value(*column))
            }
        }
    }

    fn visit_ordering(&mut self, ordering: Ordering) -> String {
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
}
