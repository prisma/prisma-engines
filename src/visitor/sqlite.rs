use crate::{ast::*, visitor::Visitor};

pub struct Sqlite {
    parameters: Vec<ParameterizedValue>,
}

impl Visitor for Sqlite {
    const C_PARAM: &'static str = "?";
    const C_QUOTE: &'static str = "`";

    fn add_parameter(&mut self, value: ParameterizedValue) {
        self.parameters.push(value);
    }

    fn visit_select(&mut self, select: Select) -> String {
        let mut result = vec!["SELECT".to_string()];

        if select.columns.is_empty() {
            result.push(String::from("*"));
        } else {
            result.push(format!("{}", self.visit_columns(select.columns)));
        }

        if let Some(table) = select.table {
            result.push(format!("FROM {}", table));

            if let Some(conditions) = select.conditions {
                result.push(format!("WHERE {}", self.visit_conditions(conditions)));
            }
            if !select.ordering.is_empty() {
                result.push(format!("ORDER BY {}", self.visit_ordering(select.ordering)));
            }
            if let Some(limit) = select.limit {
                result.push(format!("LIMIT {}", limit));
            } else {
                result.push(format!("LIMIT {}", -1));
            }
            if let Some(offset) = select.offset {
                result.push(format!("OFFSET {}", offset));
            }
        }

        result.join(" ")
    }

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue>)
    where
        Q: Into<Query>,
    {
        let mut sqlite = Sqlite {
            parameters: Vec::new(),
        };

        (
            Sqlite::visit_query(&mut sqlite, query.into()),
            sqlite.parameters,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::visitor::*;

    #[test]
    fn test_select_1() {
        let expected_sql = "SELECT ?";
        let expected_params: Vec<ParameterizedValue> = vec![1.into()];

        let query = Select::default().column(1);
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, &sql);
        assert_eq!(expected_params, params);
    }
}
