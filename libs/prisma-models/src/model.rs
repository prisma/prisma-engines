use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type ModelRef = Arc<Model>;
pub type ModelWeakRef = Weak<Model>;

#[derive(Debug, Default)]
pub struct ModelTemplate {
    pub name: String,
    pub is_embedded: bool,
    pub fields: Vec<FieldTemplate>,
    pub manifestation: Option<String>,
    pub id_field_names: Vec<String>,
    pub indexes: Vec<IndexTemplate>,
}

#[derive(DebugStub)]
pub struct Model {
    pub name: String,
    pub is_embedded: bool,

    manifestation: Option<String>,
    fields: OnceCell<Fields>,
    indexes: OnceCell<Vec<Index>>,

    #[debug_stub = "#InternalDataModelWeakRef#"]
    pub internal_data_model: InternalDataModelWeakRef,
}

impl ModelTemplate {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef) -> ModelRef {
        let model = Arc::new(Model {
            name: self.name,
            is_embedded: self.is_embedded,
            fields: OnceCell::new(),
            indexes: OnceCell::new(),
            manifestation: self.manifestation,
            internal_data_model,
        });

        let fields = Fields::new(
            self.fields
                .into_iter()
                .map(|ft| ft.build(Arc::downgrade(&model)))
                .collect(),
            Arc::downgrade(&model),
            self.id_field_names,
        );

        let indexes = self.indexes.into_iter().map(|i| i.build(&fields.scalar())).collect();

        // The model is created here and fields WILL BE UNSET before now!
        model.fields.set(fields).unwrap();
        model.indexes.set(indexes).unwrap();
        model
    }
}

impl Hash for Model {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Names are unique in the data model.
        self.name.hash(state);
    }
}

impl Eq for Model {}

impl PartialEq for Model {
    fn eq(&self, other: &Model) -> bool {
        self.name == other.name
    }
}

impl Model {
    pub(crate) fn finalize(&self) {
        self.fields.get().unwrap().finalize();
    }

    /// Returns the set of fields to be used as the primary identifier for a record of that model.
    /// The identifier is nothing but an internal convention to have an anchor point for querying, or in other words,
    /// the identifier is not to be mistaken for a stable, external identifier, but has to be understood as
    /// implementation detail that is used to reason over a fixed set of fields.
    ///
    /// The rules for determining the primary identifier are as follows:
    /// 1. If an ID definition (single or multi-part doesn't matter) is present, take that one.
    /// 2. If no ID definition is found, take the first scalar unique found that is required.
    /// 3. If no scalar unique is found, take the first compound unique found. All fields must be required.
    /// 4. If all of the above fails, we panic. Models with no unique / ID are not supported (yet).
    pub fn primary_identifier(&self) -> ModelProjection {
        let fields: Vec<_> = self
            .fields()
            .id()
            .or_else(|| {
                self.fields()
                    .scalar()
                    .into_iter()
                    .find(|sf| sf.is_unique && sf.is_required)
                    .map(|x| vec![x])
            })
            .or_else(|| {
                self.unique_indexes()
                    .into_iter()
                    .find(|index| index.fields().into_iter().all(|f| f.is_required))
                    .map(|index| index.fields().into_iter().map(|f| f.into()).collect())
            })
            .expect(&format!(
                "Unable to resolve a primary identifier for model {}.",
                self.name
            ));

        ModelProjection::new(fields.into_iter().map(Into::into).collect())
    }

    pub fn fields(&self) -> &Fields {
        self.fields
            .get()
            .ok_or_else(|| String::from("Model fields must be set!"))
            .unwrap()
    }

    pub fn indexes(&self) -> &[Index] {
        self.indexes
            .get()
            .ok_or_else(|| String::from("Model indexes must be set!"))
            .unwrap()
    }

    pub fn unique_indexes(&self) -> Vec<&Index> {
        self.indexes()
            .into_iter()
            .filter(|index| index.typ == IndexType::Unique)
            .collect()
    }

    pub fn is_legacy(&self) -> bool {
        self.internal_data_model().is_legacy()
    }

    pub fn db_name(&self) -> &str {
        self.db_name_opt().unwrap_or_else(|| self.name.as_ref())
    }

    pub fn db_name_opt(&self) -> Option<&str> {
        self.manifestation.as_ref().map(|m| m.as_ref())
    }

    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.internal_data_model
            .upgrade()
            .expect("InternalDataModel does not exist anymore. Parent internal_data_model is deleted without deleting the child internal_data_model.")
    }

    pub fn map_scalar_db_field_name(&self, name: &str) -> Option<ScalarFieldRef> {
        self.fields()
            .scalar()
            .into_iter()
            .find_map(|field| if field.db_name() == name { Some(field) } else { None })
    }
}
