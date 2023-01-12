use crate::model_extensions::ScalarFieldExt;
use itertools::Itertools;
use prisma_models::{Field, ModelProjection, RelationField, ScalarField};
use quaint::ast::{Column, Row};
use std::convert::AsRef;

pub struct ColumnIterator {
    inner: Box<dyn Iterator<Item = Column<'static>> + 'static>,
}

impl Iterator for ColumnIterator {
    type Item = Column<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl From<Vec<Column<'static>>> for ColumnIterator {
    fn from(v: Vec<Column<'static>>) -> Self {
        Self {
            inner: Box::new(v.into_iter()),
        }
    }
}

pub trait AsRow {
    fn as_row(&self) -> Row<'static>;
}

pub trait AsColumns {
    fn as_columns(&self) -> ColumnIterator;
}

impl AsColumns for ModelProjection {
    fn as_columns(&self) -> ColumnIterator {
        let cols: Vec<Column<'static>> = self
            .fields()
            .flat_map(|f| f.as_columns())
            .unique_by(|c| c.name.clone())
            .collect();

        ColumnIterator::from(cols)
    }
}

impl AsRow for ModelProjection {
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
            Field::Composite(_) => unimplemented!(),
        }
    }
}

impl AsColumns for RelationField {
    fn as_columns(&self) -> ColumnIterator {
        self.scalar_fields().as_columns()
    }
}

impl<T> AsColumns for &[T]
where
    T: AsColumn,
{
    fn as_columns(&self) -> ColumnIterator {
        let inner: Vec<_> = self.iter().map(|x| x.as_column()).collect();
        ColumnIterator::from(inner)
    }
}

impl<T> AsColumns for Vec<T>
where
    T: AsColumn,
{
    fn as_columns(&self) -> ColumnIterator {
        let inner: Vec<_> = self.iter().map(|x| x.as_column()).collect();
        ColumnIterator::from(inner)
    }
}

impl<T> AsColumn for T
where
    T: AsRef<ScalarField>,
{
    fn as_column(&self) -> Column<'static> {
        let sf = self.as_ref();

        // Unwrap is safe: SQL connectors do not anything other than models as field containers.
        let full_table_name = sf.container().as_model().unwrap().db_name_with_schema();
        let col = sf.db_name().to_string();

        let column = Column::from((full_table_name, col)).type_family(sf.type_family());
        column.default(quaint::ast::DefaultValue::Generated)
    }
}
