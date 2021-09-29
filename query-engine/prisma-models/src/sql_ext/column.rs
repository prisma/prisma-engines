use crate::{
    Field, ModelProjection, RelationField, RelationLinkManifestation, ScalarField, ScalarFieldExt, TypeIdentifier,
};
use itertools::Itertools;
use quaint::ast::{Column, Row, TypeDataLength, TypeFamily};
use std::convert::AsRef;

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
        let cols: Vec<Column<'static>> = self.iter().flat_map(AsColumns::as_columns).collect();
        ColumnIterator::from(cols)
    }
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
        }
    }
}

impl AsColumns for RelationField {
    fn as_columns(&self) -> ColumnIterator {
        let model = self.model();
        let internal_data_model = model.internal_data_model();

        let relation = self.relation();
        let table_name = if relation.is_many_to_many() {
            if let RelationLinkManifestation::RelationTable(ref rt) = relation.manifestation {
                rt.table.clone()
            } else {
                unreachable!()
            }
        } else {
            model.db_name().to_string()
        };

        let inner: Vec<_> = self
            .scalar_fields()
            .iter()
            .map(|f| {
                let parts = (
                    (internal_data_model.db_name.clone(), table_name.clone()),
                    f.db_name().to_owned(),
                );

                Column::from(parts)
            })
            .collect();

        ColumnIterator::from(inner)
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
        let db = sf.internal_data_model().db_name.clone();
        let table = sf.model().db_name().to_string();
        let col = sf.db_name().to_string();

        let parse_len = |sf: &ScalarField| {
            sf.native_type
                .as_ref()
                .and_then(|nt| nt.args.first())
                .and_then(|len| match len.as_str() {
                    "max" | "Max" | "MAX" | "MaX" | "mAx" | "maX" => Some(TypeDataLength::Maximum),
                    num => num.parse().map(TypeDataLength::Constant).ok(),
                })
        };

        let type_family = match sf.type_identifier {
            TypeIdentifier::String => TypeFamily::Text(parse_len(sf)),
            TypeIdentifier::Int => TypeFamily::Int,
            TypeIdentifier::BigInt => TypeFamily::Int,
            TypeIdentifier::Float => TypeFamily::Double,
            TypeIdentifier::Decimal => {
                let params = sf
                    .native_type
                    .as_ref()
                    .map(|nt| nt.args.iter())
                    .and_then(|mut args| Some((args.next()?, args.next()?)))
                    .and_then(|(p, s)| Some((p.parse::<u8>().ok()?, s.parse::<u8>().ok()?)));

                TypeFamily::Decimal(params)
            }
            TypeIdentifier::Boolean => TypeFamily::Boolean,
            TypeIdentifier::Enum(_) => TypeFamily::Text(Some(TypeDataLength::Constant(8000))),
            TypeIdentifier::UUID => TypeFamily::Uuid,
            TypeIdentifier::Json => TypeFamily::Text(Some(TypeDataLength::Maximum)),
            TypeIdentifier::Xml => TypeFamily::Text(Some(TypeDataLength::Maximum)),
            TypeIdentifier::DateTime => TypeFamily::DateTime,
            TypeIdentifier::Bytes => TypeFamily::Text(parse_len(sf)),
            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
        };

        let column = Column::from(((db, table), col)).type_family(type_family);

        match sf.default_value.as_ref().and_then(|d| d.get()) {
            Some(default) => column.default(sf.value(default)),
            None => column.default(quaint::ast::DefaultValue::Generated),
        }
    }
}
