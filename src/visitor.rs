use crate::ast::*;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub trait Visitor {
    const C_PARAM: &'static str;
    const C_QUOTE: &'static str;

    fn add_parameter(&mut self, value: ParameterizedValue);

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

    fn visit_column(column: Column) -> String {
        let format_parts = |parts: Vec<String>| {
            let mut result = Vec::new();

            for part in parts.into_iter() {
                result.push(format!("{}{}{}", Self::C_QUOTE, part, Self::C_QUOTE));
            }

            result.join(".")
        };

        match (column.database, column.table) {
            (Some(db), Some(table)) => format_parts(vec![db, table, column.name]),
            (None, Some(table)) => format_parts(vec![table, column.name]),
            _ => format_parts(vec![column.name]),
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
            Expression::Like(like) => self.visit_like(*like),
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
            Compare::Null(column) => format!("{} IS NULL", self.visit_database_value(*column)),
            Compare::NotNull(column) => {
                format!("{} IS NOT NULL", self.visit_database_value(*column))
            }
        }
    }

    fn visit_like(&mut self, like: Like) -> String {
        let expression = self.visit_expression(like.expression);

        match like.typ {
            LikeType::Like => {
                self.add_parameter(ParameterizedValue::Text(format!("%{}%", like.value)));
                format!("{} LIKE ?", expression)
            }
            LikeType::NotLike => {
                self.add_parameter(ParameterizedValue::Text(format!("%{}%", like.value)));
                format!("{} NOT LIKE ?", expression)
            }
            LikeType::StartsWith => {
                self.add_parameter(ParameterizedValue::Text(format!("{}%", like.value)));
                format!("{} LIKE ?", expression)
            }
            LikeType::NotStartsWith => {
                self.add_parameter(ParameterizedValue::Text(format!("{}%", like.value)));
                format!("{} NOT LIKE ?", expression)
            }
            LikeType::EndsWith => {
                self.add_parameter(ParameterizedValue::Text(format!("%{}", like.value)));
                format!("{} LIKE ?", expression)
            }
            LikeType::NotEndsWith => {
                self.add_parameter(ParameterizedValue::Text(format!("%{}", like.value)));
                format!("{} NOT LIKE ?", expression)
            }
        }
    }

    fn visit_ordering(&mut self, ordering: Ordering) -> String {
        let mut result = Vec::new();

        for (value, ordering) in ordering.0.into_iter() {
            let direction = ordering.map(|dir| match dir {
                Order::Ascending => " ASC",
                Order::Descending => " DESC",
            });

            result.push(format!(
                "{}{}",
                self.visit_database_value(value),
                direction.unwrap_or("")
            ));
        }

        result.join(", ")
    }

    fn visit_select(&mut self, select: Select) -> String;
    fn build(self, query: Query) -> (String, Vec<ParameterizedValue>);
}
