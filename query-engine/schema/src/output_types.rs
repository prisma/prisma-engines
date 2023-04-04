use super::*;
use fmt::Debug;
use once_cell::sync::OnceCell;
use prisma_models::{ast::ModelId, ModelRef};
use std::{fmt, sync::Arc};

#[derive(Debug, Clone)]
pub enum OutputType {
    Enum(EnumTypeId),
    List(OutputTypeRef),
    Object(OutputObjectTypeId),
    Scalar(ScalarType),
}

impl OutputType {
    pub fn list(containing: OutputType) -> OutputType {
        OutputType::List(Arc::new(containing))
    }

    pub fn object(containing: OutputObjectTypeId) -> OutputType {
        OutputType::Object(containing)
    }

    pub fn string() -> OutputType {
        OutputType::Scalar(ScalarType::String)
    }

    pub fn int() -> OutputType {
        OutputType::Scalar(ScalarType::Int)
    }

    pub fn bigint() -> OutputType {
        OutputType::Scalar(ScalarType::BigInt)
    }

    pub fn float() -> OutputType {
        OutputType::Scalar(ScalarType::Float)
    }

    pub fn decimal() -> OutputType {
        OutputType::Scalar(ScalarType::Decimal)
    }

    pub fn boolean() -> OutputType {
        OutputType::Scalar(ScalarType::Boolean)
    }

    pub fn enum_type(containing: EnumTypeId) -> OutputType {
        OutputType::Enum(containing)
    }

    pub fn date_time() -> OutputType {
        OutputType::Scalar(ScalarType::DateTime)
    }

    pub fn json() -> OutputType {
        OutputType::Scalar(ScalarType::Json)
    }

    pub fn uuid() -> OutputType {
        OutputType::Scalar(ScalarType::UUID)
    }

    pub fn xml() -> OutputType {
        OutputType::Scalar(ScalarType::Xml)
    }

    pub fn bytes() -> OutputType {
        OutputType::Scalar(ScalarType::Bytes)
    }

    /// Attempts to recurse through the type until an object type is found.
    /// Returns Some(ObjectTypeStrongRef) if ab object type is found, None otherwise.
    pub fn as_object_type<'a>(&self, db: &'a QuerySchemaDatabase) -> Option<(OutputObjectTypeId, &'a ObjectType)> {
        match self {
            OutputType::Enum(_) => None,
            OutputType::List(inner) => inner.as_object_type(db),
            OutputType::Object(obj) => Some((*obj, &db[*obj])),
            OutputType::Scalar(_) => None,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, OutputType::List(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, OutputType::Object(_))
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, OutputType::Scalar(_))
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, OutputType::Enum(_))
    }

    pub fn is_scalar_list(&self) -> bool {
        match self {
            OutputType::List(typ) => typ.is_scalar(),
            _ => false,
        }
    }

    pub fn is_enum_list(&self) -> bool {
        match self {
            OutputType::List(typ) => typ.is_enum(),
            _ => false,
        }
    }
}

pub struct ObjectType {
    pub identifier: Identifier,
    fields: OnceCell<Vec<OutputField>>,

    // Object types can directly map to models.
    model: Option<ModelId>,
}

impl Debug for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectType")
            .field("identifier", &self.identifier)
            .field("fields", &"#Fields Cell#")
            .field("model", &self.model)
            .finish()
    }
}

impl ObjectType {
    pub fn new(ident: Identifier, model: Option<ModelId>) -> Self {
        Self {
            identifier: ident,
            fields: OnceCell::new(),
            model,
        }
    }

    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn add_field(&mut self, field: OutputField) {
        self.fields.get_mut().unwrap().push(field)
    }

    pub fn get_fields(&self) -> &[OutputField] {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<OutputField>) {
        self.fields.set(fields).unwrap();
    }

    pub fn find_field<'a>(&'a self, name: &str) -> Option<(usize, &'a OutputField)> {
        self.get_fields().iter().enumerate().find(|(_, f)| f.name == name)
    }

    /// True if fields are empty, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.get_fields().is_empty()
    }
}

#[derive(Debug)]
pub struct OutputField {
    pub name: String,
    pub field_type: OutputTypeRef,
    pub deprecation: Option<Deprecation>,

    /// Arguments are input fields, but positioned in context of an output field
    /// instead of being attached to an input object.
    pub arguments: Vec<InputField>,

    /// Indicates the presence of the field on the higher output objects.
    /// States whether or not the field can be null.
    pub is_nullable: bool,

    /// Relevant for resolving top level queries.
    pub query_info: Option<QueryInfo>,
}

impl OutputField {
    pub fn nullable(mut self) -> Self {
        self.is_nullable = true;
        self
    }

    pub fn nullable_if(self, condition: bool) -> Self {
        if condition {
            self.nullable()
        } else {
            self
        }
    }

    pub fn deprecate<T, S>(mut self, reason: T, since_version: S, planned_removal_version: Option<String>) -> Self
    where
        T: Into<String>,
        S: Into<String>,
    {
        self.deprecation = Some(Deprecation {
            reason: reason.into(),
            since_version: since_version.into(),
            planned_removal_version,
        });

        self
    }

    pub fn model(&self) -> Option<&ModelRef> {
        self.query_info.as_ref().and_then(|info| info.model.as_ref())
    }

    pub fn is_find_unique(&self) -> bool {
        matches!(self.query_tag(), Some(&QueryTag::FindUnique))
    }

    pub fn query_info(&self) -> Option<&QueryInfo> {
        self.query_info.as_ref()
    }

    pub fn query_tag(&self) -> Option<&QueryTag> {
        self.query_info().map(|info| &info.tag)
    }

    // Is relation determines whether the given output field maps to a a relation, i.e.
    // is an object and that object is backed by a model, meaning that it is not an scalar list
    pub fn maps_to_relation(&self, query_schema: &QuerySchema) -> bool {
        let o = self.field_type.as_object_type(&query_schema.db);
        o.is_some() && o.unwrap().1.model.is_some()
    }
}
