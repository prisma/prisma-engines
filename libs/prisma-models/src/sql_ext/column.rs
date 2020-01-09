use crate::{Field, RelationField, ScalarField, ModelIdentifier};
use quaint::ast::{Column, Row};

pub trait AsRow {
    fn as_row(&self) -> Row<'static>;
}

pub trait AsColumns {
    fn as_columns(&self) -> Vec<Column<'static>>;
}

impl AsColumns for &[Field] {
    fn as_columns(&self) -> Vec<Column<'static>> {
        self.into_iter().map(|f| f.as_column()).collect()
    }
}

impl AsColumns for ModelIdentifier {
    fn as_columns(&self) -> Vec<Column<'static>> {
        self.fields().as_columns()
    }
}

impl AsRow for ModelIdentifier {
    fn as_row(&self) -> Row<'static> {
        Row::from(self.as_columns())
    }
}

pub trait AsColumn {
    fn as_column(&self) -> Column<'static>;
}

impl AsColumn for Field {
    fn as_column(&self) -> Column<'static> {
        match self {
            Field::Scalar(ref sf) => sf.as_column(),
            Field::Relation(ref rf) => rf.as_column(),
        }
    }
}

impl AsColumn for RelationField {
    fn as_column(&self) -> Column<'static> {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        let db_name = self.db_name();
        let parts = (
            (internal_data_model.db_name.clone(), model.db_name().to_string()),
            db_name.clone(),
        );

        parts.into()
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
