use crate::{Field, FieldWeak, ScalarFieldRef};

#[derive(Debug)]
pub struct IndexTemplate {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub typ: IndexType,
}

impl IndexTemplate {
    pub fn build(self, fields: &[Field]) -> Index {
        let fields = match self.typ {
            IndexType::Unique => Self::map_fields(self.fields, fields),
            IndexType::Normal => vec![],
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }

    fn map_fields(field_names: Vec<String>, fields: &[Field]) -> Vec<FieldWeak> {
        field_names
            .into_iter()
            .map(|name| {
                let field = fields
                    .iter()
                    .find(|sf| sf.name() == name)
                    .expect(&format!("Unable to resolve field '{}'", name));

                field.downgrade()
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<FieldWeak>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> Vec<Field> {
        self.fields.iter().map(|sf| sf.upgrade()).collect()
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        self.fields
            .iter()
            .map(|f| f.upgrade())
            .filter_map(Field::as_scalar)
            .collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
