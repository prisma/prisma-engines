use crate::{Field, ModelIdentifier, RelationField, ScalarField};
use quaint::ast::{Column, Row};

pub struct ColumnIterator {
    count: usize,
    inner: Box<dyn Iterator<Item = Column<'static>> + 'static>,
}

impl ColumnIterator {
    pub fn new(inner: impl Iterator<Item = Column<'static>> + 'static, count: usize) -> Self {
        Self {
            inner: Box::new(inner),
            count,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Iterator for ColumnIterator {
    type Item = Column<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl From<Vec<Column<'static>>> for ColumnIterator {
    fn from(v: Vec<Column<'static>>) -> Self {
        let count = v.len();

        Self {
            inner: Box::new(v.into_iter()),
            count,
        }
    }
}

pub trait AsRow {
    fn as_row(&self) -> Row<'static>;
}

pub trait AsColumns {
    fn as_columns(&self) -> ColumnIterator;
}

impl AsColumns for &[Field] {
    fn as_columns(&self) -> ColumnIterator {
        let cols: Vec<Column<'static>> = self.into_iter().flat_map(AsColumns::as_columns).collect();
        ColumnIterator::from(cols)
    }
}

impl AsColumns for ModelIdentifier {
    fn as_columns(&self) -> ColumnIterator {
        let cols: Vec<Column<'static>> = self.fields().flat_map(|f| f.as_columns()).collect();
        ColumnIterator::from(cols)
    }
}

impl AsRow for ModelIdentifier {
    fn as_row(&self) -> Row<'static> {
        let cols: Vec<Column<'static>> = self.as_columns().collect();
        Row::from(cols)
    }
}

pub trait AsColumn {
    fn as_column(&self) -> Column<'static>;
}

impl AsColumns for Field {
    fn as_columns(&self) -> ColumnIterator {
        match self {
            Field::Scalar(ref sf) => ColumnIterator::from(vec![sf.as_column()]),
            Field::Relation(ref rf) => rf.as_columns(),
        }
    }
}

impl AsColumns for RelationField {
    fn as_columns(&self) -> ColumnIterator {
        let model = self.model();
        let internal_data_model = model.internal_data_model();

        let inner: Vec<_> = self
            .data_source_fields()
            .iter()
            .map(|dsf| {
                let parts = (
                    (internal_data_model.db_name.clone(), model.db_name().to_string()),
                    dsf.name.clone(),
                );

                Column::from(parts)
            })
            .collect();

        ColumnIterator::from(inner)
    }
}

impl AsColumns for &[crate::field::DataSourceFieldRef] {
    fn as_columns(&self) -> ColumnIterator {
        let inner: Vec<_> = self.iter().map(|dsf| dsf.as_column()).collect();
        ColumnIterator::from(inner)
    }
}

impl AsColumn for ScalarField {
    fn as_column(&self) -> Column<'static> {
        let db = self.internal_data_model().db_name.clone();
        let table = self.model().db_name().to_string();
        let col = self.db_name().to_string();

        Column::from(((db, table), col))
    }
}

impl AsColumn for crate::field::DataSourceField {
    fn as_column(&self) -> Column<'static> {
        let model = self.model_field().model();
        let db = model.internal_data_model().db_name.clone();
        let table = model.db_name().to_string();
        let col = self.name.to_string();

        Column::from(((db, table), col))
    }
}
