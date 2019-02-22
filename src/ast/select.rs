use crate::ast::*;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Select {
    pub table: Option<Table>,
    pub columns: Vec<DatabaseValue>,
    pub conditions: Option<ConditionTree>,
    pub ordering: Ordering,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Into<DatabaseValue> for Select {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Select(self)
    }
}

impl Into<Query> for Select {
    fn into(self) -> Query {
        Query::Select(self)
    }
}

impl Select {
    pub fn from<T>(table: T) -> Self
    where
        T: Into<Table>,
    {
        Select {
            table: Some(table.into()),
            ..Default::default()
        }
    }

    pub fn value<T>(mut self, value: T) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.columns.push(value.into());
        self
    }

    pub fn column<T>(mut self, column: T) -> Self
    where
        T: Into<Column>,
    {
        self.columns.push(column.into().into());
        self
    }

    pub fn columns(mut self, columns: Vec<DatabaseValue>) -> Self {
        self.columns = columns;
        self
    }

    pub fn so_that<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree>,
    {
        self.conditions = Some(conditions.into());
        self
    }

    pub fn order_by<T>(mut self, value: OrderDefinition) -> Self {
        self.ordering = self.ordering.append(value);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}
