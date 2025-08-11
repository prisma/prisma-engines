use std::borrow::Cow;

use indexmap::IndexMap;
use query_structure::{FieldTypeInformation, PrismaValueType, TypeIdentifier};
use serde::Serialize;

use crate::expression::EnumsMap;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResultNode {
    AffectedRows,
    Object(Object),
    #[serde(rename_all = "camelCase")]
    Value {
        db_name: Cow<'static, str>,
        result_type: PrismaValueType,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    serialized_name: Option<Cow<'static, str>>,
    fields: IndexMap<Cow<'static, str>, ResultNode>,
    skip_nulls: bool,
}

impl Object {
    fn new(serialized_name: Option<impl Into<Cow<'static, str>>>) -> Self {
        Self {
            serialized_name: serialized_name.map(Into::into),
            fields: IndexMap::new(),
            skip_nulls: false,
        }
    }

    fn set_skip_nulls(&mut self, skip: bool) -> &mut Self {
        self.skip_nulls = skip;
        self
    }

    pub fn serialized_name(&self) -> Option<&str> {
        self.serialized_name.as_deref()
    }

    pub fn fields(&self) -> &IndexMap<Cow<'static, str>, ResultNode> {
        &self.fields
    }
}

pub struct ResultNodeBuilder<'a> {
    enums: &'a mut EnumsMap,
}

impl<'a> ResultNodeBuilder<'a> {
    pub fn new(enums: &'a mut EnumsMap) -> Self {
        Self { enums }
    }

    pub fn new_object(serialized_name: Option<impl Into<Cow<'static, str>>>) -> ObjectBuilder {
        ObjectBuilder::new(serialized_name)
    }

    #[inline]
    pub fn new_value(
        &mut self,
        db_name: impl Into<Cow<'static, str>>,
        result_type: FieldTypeInformation,
    ) -> ResultNode {
        self.new_value_inner(db_name.into(), result_type)
    }

    fn new_value_inner(&mut self, db_name: Cow<'static, str>, result_type: FieldTypeInformation) -> ResultNode {
        let prisma_type = result_type.to_prisma_type();
        if let TypeIdentifier::Enum(id) = result_type.typ.id {
            self.enums.add(result_type.typ.dm.zip(id));
        }
        ResultNode::Value {
            db_name,
            result_type: prisma_type,
        }
    }
}

pub struct ObjectBuilder {
    object: Object,
}

impl ObjectBuilder {
    fn new(serialized_name: Option<impl Into<Cow<'static, str>>>) -> Self {
        Self {
            object: Object::new(serialized_name),
        }
    }

    pub fn set_skip_nulls(&mut self, skip: bool) -> &mut Self {
        self.object.set_skip_nulls(skip);
        self
    }

    pub fn add_field(&mut self, key: impl Into<Cow<'static, str>>, node: ResultNode) {
        ObjectMutBuilder::new(&mut self.object).add_field(key, node)
    }

    pub fn entry_or_insert_nested(&mut self, key: impl Into<Cow<'static, str>> + Clone) -> ObjectMutBuilder<'_> {
        self.entry_or_insert(key.clone(), Some(key))
    }

    #[inline]
    pub fn entry_or_insert(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        original_key: Option<impl Into<Cow<'static, str>>>,
    ) -> ObjectMutBuilder<'_> {
        self.entry_or_insert_inner(key.into(), original_key.map(Into::into))
    }

    fn entry_or_insert_inner(
        &mut self,
        key: Cow<'static, str>,
        original_key: Option<Cow<'static, str>>,
    ) -> ObjectMutBuilder<'_> {
        let node = self
            .object
            .fields
            .entry(key)
            .or_insert(ResultNode::Object(Object::new(original_key)));

        let ResultNode::Object(object) = node else {
            panic!("ObjectBuilder::entry_or_insert can only be called with key which is vacant or points at an object")
        };

        ObjectMutBuilder::new(object)
    }

    pub fn build(self) -> ResultNode {
        ResultNode::Object(self.object)
    }
}

pub struct ObjectMutBuilder<'a> {
    object: &'a mut Object,
}

impl<'a> ObjectMutBuilder<'a> {
    fn new(object: &'a mut Object) -> Self {
        Self { object }
    }

    pub fn add_field(&mut self, key: impl Into<Cow<'static, str>>, node: ResultNode) {
        self.object.fields.insert(key.into(), node);
    }
}
