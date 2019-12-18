use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};
use uuid::Uuid;

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
                .map(|fi| fi.build(Arc::downgrade(&model)))
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
    pub fn generate_id(&self) -> Option<RecordIdentifier> {
        self.fields().id().map(|mut id_fields| {
            if id_fields.len() == 1 {
                let id = id_fields.pop().unwrap();
                let generated: PrismaValue = match id.type_identifier {
                    // This will panic when:
                    //
                    // - System time goes backwards
                    // - There is an error generating a fingerprint
                    // - Time cannot be converted to a string.
                    //
                    // Panic is a better choice than bubbling this up
                    TypeIdentifier::GraphQLID => GraphqlId::String(cuid::cuid().unwrap()),
                    TypeIdentifier::UUID => GraphqlId::UUID(Uuid::new_v4()),
                    TypeIdentifier::Int => panic!("Cannot generate integer ids."),

                    // All other ID types are rejected on data model construction.
                    _ => unreachable!(),
                }
                .into();

                (Field::Scalar(id), generated).into()
            } else {
                // Todo: Check what our plan is regarding this.
                unimplemented!("Generated multi-part IDs are currently unsupported.")
            }
        })
    }

    /// Returns the set of fields to be used as the primary identifier for a record of that model.
    /// The implementation guarantees that the returned set of fields is deterministic for the same underlying data model.
    /// The rules for finding a primary identifier are as follows:
    /// 1. If an ID definition (single or multi-part doesn't matter) is present, take that one.
    /// 2. If no ID definition is found, take the first scalar unique found.
    /// 3. If no scalar unique is found, take the first compound unique found.
    /// 4. If all of the above fails, we panic. Models with no unique / ID are not supported (yet).
    ///
    /// This relies entirely on the datamodel parsing and conversion to have a stable ordering of fields.
    pub fn primary_identifier(&self) -> Vec<Field> {
        self.fields()
            .id()
            .map(|fields| fields.into_iter().map(|f| Field::Scalar(f)).collect())
            .or_else(|| {
                self.fields()
                    .scalar()
                    .into_iter()
                    .find(|sf| sf.is_unique)
                    .map(|x| vec![Field::Scalar(x)])
            })
            .or_else(|| {
                self.unique_indexes()
                    .first()
                    .map(|index| index.fields().into_iter().map(|f| Field::Scalar(f)).collect())
            })
            .expect(&format!(
                "Unable to resolve a primary identifier for model {}.",
                self.name
            ))
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
}
