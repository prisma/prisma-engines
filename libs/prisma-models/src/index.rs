use crate::ScalarFieldRef;
use crate::{Field, FieldWeak, Fields};
use failure::ResultExt;

#[derive(Debug)]
pub struct IndexTemplate {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub typ: IndexType,
}

impl IndexTemplate {
    pub fn build(self, fields: &Fields) -> Index {
        let fields = match self.typ {
            IndexType::Unique => self
                .fields
                .into_iter()
                .map(|name| {
                    let field = fields
                        .find_from_all(&name)
                        .with_context(|err| format!("Unable to resolve scalar field '{}'. {}", name, err))
                        .unwrap();

                    field.downgrade()
                })
                .collect(),

            IndexType::Normal => vec![],
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
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
        self.fields.iter().map(|f| f.upgrade()).collect()
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        self.fields
            .iter()
            .filter_map(|f| match f {
                FieldWeak::Scalar(s) => Some(s),
                _ => None,
            })
            .map(|f| f.upgrade().unwrap())
            .collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
