use crate::ast::ParameterizedValue;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Row {
    values: Vec<ParameterizedValue>,
}

impl Row {
    pub fn new() -> Self {
        Row { values: Vec::new() }
    }

    pub fn add<T>(mut self, value: T) -> Self
    where
        T: Into<ParameterizedValue>,
    {
        self.values.push(value.into());
        self
    }
}

impl<T> From<Vec<T>> for Row
where
    T: Into<ParameterizedValue>,
{
    fn from(vector: Vec<T>) -> Row {
        vector
            .into_iter()
            .fold(Row::new(), |row, v| row.add(v.into()))
    }
}

impl<A, B> From<(A, B)> for Row
where
    A: Into<ParameterizedValue>,
    B: Into<ParameterizedValue>,
{
    fn from(vals: (A, B)) -> Row {
        Row::new().add(vals.0).add(vals.1)
    }
}

impl<A, B, C> From<(A, B, C)> for Row
where
    A: Into<ParameterizedValue>,
    B: Into<ParameterizedValue>,
    C: Into<ParameterizedValue>,
{
    fn from(vals: (A, B, C)) -> Row {
        Row::new().add(vals.0).add(vals.1).add(vals.2)
    }
}

impl<A, B, C, D> From<(A, B, C, D)> for Row
where
    A: Into<ParameterizedValue>,
    B: Into<ParameterizedValue>,
    C: Into<ParameterizedValue>,
    D: Into<ParameterizedValue>,
{
    fn from(vals: (A, B, C, D)) -> Row {
        Row::new().add(vals.0).add(vals.1).add(vals.2).add(vals.3)
    }
}

impl<A, B, C, D, E> From<(A, B, C, D, E)> for Row
where
    A: Into<ParameterizedValue>,
    B: Into<ParameterizedValue>,
    C: Into<ParameterizedValue>,
    D: Into<ParameterizedValue>,
    E: Into<ParameterizedValue>,
{
    fn from(vals: (A, B, C, D, E)) -> Row {
        Row::new()
            .add(vals.0)
            .add(vals.1)
            .add(vals.2)
            .add(vals.3)
            .add(vals.4)
    }
}
