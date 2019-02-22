use crate::{ast::*, visitor::Visitor};

pub struct Sqlite {
    parameters: Vec<ParameterizedValue>,
}

impl Sqlite {
    pub fn new() -> Sqlite {
        Sqlite {
            parameters: Vec::new(),
        }
    }
}

impl Visitor for Sqlite {
    const C_PARAM: &'static str = "?";
    const C_QUOTE: &'static str = "`";

    fn add_parameter(&mut self, value: ParameterizedValue) {
        self.parameters.push(value);
    }

    fn visit_select(&mut self, select: Select) -> String {
        let mut result = vec!["Select".to_string()];

        if select.columns.is_empty() {
            result.push(String::from("*"));
        } else {
            result.push(format!("{}", self.visit_columns(select.columns)));
        }
        if let Some(table) = select.table {
            result.push(format!("FROM {}", table));
        }
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

        result.join(" ")
    }

    fn build(mut self, query: Query) -> (String, Vec<ParameterizedValue>) {
        (Sqlite::visit_query(&mut self, query), self.parameters)
    }
}
