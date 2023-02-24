use crate::{model_extensions::ScalarFieldExt, Context};
use itertools::Itertools;
use prisma_models::{Field, ModelProjection, RelationField, ScalarField};
use quaint::ast::{Column, Row};

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

pub(crate) trait AsRow {
    fn as_row(&self, ctx: &Context<'_>) -> Row<'static>;
}

pub(crate) trait AsColumns {
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator;
}

impl AsColumns for ModelProjection {
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        let cols: Vec<Column<'static>> = self
            .fields()
            .flat_map(|f| f.as_columns(ctx))
            .unique_by(|c| c.name.clone())
            .collect();

        ColumnIterator::from(cols)
    }
}

impl AsRow for ModelProjection {
    fn as_row(&self, ctx: &Context<'_>) -> Row<'static> {
        let cols: Vec<Column<'static>> = self.as_columns(ctx).collect();
        Row::from(cols)
    }
}

pub(crate) trait AsColumn {
    fn as_column(&self, ctx: &Context<'_>) -> Column<'static>;
}

impl AsColumns for Field {
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        match self {
            Field::Scalar(ref sf) => ColumnIterator::from(vec![sf.as_column(ctx)]),
            Field::Relation(ref rf) => rf.as_columns(ctx),
            Field::Composite(_) => unimplemented!(),
        }
    }
}

impl AsColumns for RelationField {
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        self.scalar_fields().as_columns(ctx)
    }
}

impl<T> AsColumns for &[T]
where
    T: AsColumn,
{
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        let inner: Vec<_> = self.iter().map(|x| x.as_column(ctx)).collect();
        ColumnIterator::from(inner)
    }
}

impl<T> AsColumns for Vec<T>
where
    T: AsColumn,
{
    fn as_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        let inner: Vec<_> = self.iter().map(|x| x.as_column(ctx)).collect();
        ColumnIterator::from(inner)
    }
}

impl AsColumn for ScalarField {
    fn as_column(&self, ctx: &Context<'_>) -> Column<'static> {
        // Unwrap is safe: SQL connectors do not anything other than models as field containers.
        let full_table_name = super::table::db_name_with_schema(&self.container().as_model().unwrap(), ctx);
        let col = self.db_name().to_string();

        let column = Column::from((full_table_name, col)).type_family(self.type_family());
        column.default(quaint::ast::DefaultValue::Generated)
    }
}
