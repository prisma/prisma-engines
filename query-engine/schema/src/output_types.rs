use super::*;
use fmt::Debug;
use once_cell::sync::Lazy;
use psl::parser_database as db;
use std::{borrow::Cow, fmt};

#[derive(Debug, Clone)]
pub struct OutputType<'a> {
    is_list: bool,
    pub inner: InnerOutputType<'a>,
}

#[derive(Debug, Clone)]
pub enum InnerOutputType<'a> {
    Enum(EnumType),
    Object(ObjectType<'a>),
    Scalar(ScalarType),
}

impl<'a> OutputType<'a> {
    pub fn non_list(inner: InnerOutputType<'a>) -> Self {
        OutputType { is_list: false, inner }
    }

    pub(crate) fn list(containing: InnerOutputType<'a>) -> Self {
        OutputType {
            is_list: true,
            inner: containing,
        }
    }

    pub fn object(containing: ObjectType<'a>) -> Self {
        OutputType::non_list(InnerOutputType::Object(containing))
    }

    pub(crate) fn string() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::String)
    }

    pub(crate) fn int() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Int)
    }

    pub(crate) fn bigint() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::BigInt)
    }

    pub(crate) fn float() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Float)
    }

    pub(crate) fn decimal() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Decimal)
    }

    pub(crate) fn boolean() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Boolean)
    }

    pub(crate) fn enum_type(containing: EnumType) -> InnerOutputType<'a> {
        InnerOutputType::Enum(containing)
    }

    pub(crate) fn date_time() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::DateTime)
    }

    pub(crate) fn json() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Json)
    }

    pub(crate) fn uuid() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::UUID)
    }

    pub(crate) fn bytes() -> InnerOutputType<'a> {
        InnerOutputType::Scalar(ScalarType::Bytes)
    }

    /// Attempts to recurse through the type until an object type is found.
    /// Returns Some(ObjectTypeStrongRef) if ab object type is found, None otherwise.
    pub fn as_object_type<'b>(&'b self) -> Option<&'b ObjectType<'a>> {
        match &self.inner {
            InnerOutputType::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn is_list(&self) -> bool {
        self.is_list
    }

    pub fn is_object(&self) -> bool {
        matches!(self.inner, InnerOutputType::Object(_))
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.inner, InnerOutputType::Scalar(_))
    }

    pub fn is_enum(&self) -> bool {
        matches!(self.inner, InnerOutputType::Enum(_))
    }

    pub fn is_scalar_list(&self) -> bool {
        self.is_list() && self.is_scalar()
    }

    pub fn is_enum_list(&self) -> bool {
        self.is_list() && self.is_enum()
    }
}

type OutputObjectFields<'a> =
    Arc<Lazy<Vec<OutputField<'a>>, Box<dyn FnOnce() -> Vec<OutputField<'a>> + Send + Sync + 'a>>>;

#[derive(Clone)]
pub struct ObjectType<'a> {
    pub(crate) identifier: Identifier,
    pub(crate) fields: OutputObjectFields<'a>,

    // Object types can directly map to models.
    pub(crate) model: Option<db::ModelId>,
}

impl Debug for ObjectType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectType")
            .field("identifier", &self.identifier)
            .field("model", &self.model)
            .finish()
    }
}

impl<'a> ObjectType<'a> {
    pub(crate) fn new(
        identifier: Identifier,
        fields: impl FnOnce() -> Vec<OutputField<'a>> + Send + Sync + 'a,
    ) -> Self {
        let lazy = Lazy::<Vec<OutputField<'_>>, _>::new(
            Box::new(fields) as Box<dyn FnOnce() -> Vec<OutputField<'a>> + Send + Sync + 'a>
        );
        ObjectType {
            identifier,
            fields: Arc::new(lazy),
            model: None,
        }
    }

    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn name(&self) -> String {
        self.identifier.name()
    }

    pub fn get_fields(&self) -> &[OutputField<'a>] {
        (*self.fields).as_ref()
    }

    pub fn find_field(&self, name: &str) -> Option<&OutputField<'a>> {
        self.get_fields().iter().find(|f| f.name == name)
    }
}

type OutputFieldArguments<'a> =
    Option<Arc<Lazy<Vec<InputField<'a>>, Box<dyn FnOnce() -> Vec<InputField<'a>> + Send + Sync + 'a>>>>;

#[derive(Clone)]
pub struct OutputField<'a> {
    pub(crate) name: Cow<'a, str>,
    pub field_type: OutputType<'a>,

    /// Arguments are input fields, but positioned in context of an output field
    /// instead of being attached to an input object.
    pub(super) arguments: OutputFieldArguments<'a>,

    /// Indicates the presence of the field on the higher output objects.
    /// States whether or not the field can be null.
    pub is_nullable: bool,

    pub(super) query_info: Option<QueryInfo>,
}

impl Debug for OutputField<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputField<'_>")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<'a> OutputField<'a> {
    pub fn name(&self) -> &Cow<'a, str> {
        &self.name
    }

    pub fn arguments(&self) -> &[InputField<'a>] {
        self.arguments.as_ref().map(|f| (**f).as_slice()).unwrap_or(&[])
    }

    pub(crate) fn nullable(mut self) -> Self {
        self.is_nullable = true;
        self
    }

    pub(crate) fn nullable_if(self, condition: bool) -> Self {
        if condition {
            self.nullable()
        } else {
            self
        }
    }

    pub fn model(&self) -> Option<db::ModelId> {
        self.query_info.as_ref().and_then(|info| info.model)
    }

    pub fn field_type(&self) -> &OutputType<'a> {
        &self.field_type
    }

    pub fn is_find_unique(&self) -> bool {
        matches!(
            self.query_tag(),
            Some(&QueryTag::FindUnique | QueryTag::FindUniqueOrThrow)
        )
    }

    /// Relevant for resolving top level queries.
    pub fn query_info(&self) -> Option<&QueryInfo> {
        self.query_info.as_ref()
    }

    pub fn query_tag(&self) -> Option<&QueryTag> {
        self.query_info().map(|info| &info.tag)
    }

    // Is relation determines whether the given output field maps to a a relation, i.e.
    // is an object and that object is backed by a model, meaning that it is not an scalar list
    pub fn maps_to_relation(&self) -> bool {
        let o = self.field_type.as_object_type();
        o.is_some() && o.unwrap().model.is_some()
    }
}
