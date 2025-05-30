use indexmap::IndexMap;
use query_structure::{PrismaValueType, ScalarFieldResultType};
use serde::Serialize;

use crate::expression::EnumsMap;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ResultNode {
    AffectedRows,
    Object(Object),
    #[serde(rename_all = "camelCase")]
    Value {
        db_name: String,
        result_type: PrismaValueType,
    },
}

#[derive(Debug, Serialize)]
pub struct Object {
    flattened: bool,
    fields: IndexMap<String, ResultNode>,
}

impl Object {
    fn new(kind: ObjectKind) -> Self {
        Self {
            flattened: kind.is_flattened(),
            fields: IndexMap::new(),
        }
    }

    pub fn is_flattened(&self) -> bool {
        self.flattened
    }

    pub fn fields(&self) -> &IndexMap<String, ResultNode> {
        &self.fields
    }
}

pub enum ObjectKind {
    Flattened,
    Nested,
}

impl ObjectKind {
    fn is_flattened(&self) -> bool {
        matches!(self, ObjectKind::Flattened)
    }
}

pub struct ResultNodeBuilder<'a> {
    enums: &'a mut EnumsMap,
}

impl<'a> ResultNodeBuilder<'a> {
    pub fn new(enums: &'a mut EnumsMap) -> Self {
        Self { enums }
    }

    pub fn new_object() -> ObjectBuilder {
        ObjectBuilder::new(ObjectKind::Nested)
    }

    pub fn new_flattened_object() -> ObjectBuilder {
        ObjectBuilder::new(ObjectKind::Flattened)
    }

    pub fn new_value(&mut self, db_name: String, result_type: ScalarFieldResultType) -> ResultNode {
        self.enums.add(&result_type.typ);
        ResultNode::Value {
            db_name,
            result_type: result_type.to_prisma_type(),
        }
    }
}

pub struct ObjectBuilder {
    object: Object,
}

impl ObjectBuilder {
    fn new(kind: ObjectKind) -> Self {
        Self {
            object: Object::new(kind),
        }
    }

    pub fn add_field(&mut self, key: impl Into<String>, node: ResultNode) {
        ObjectMutBuilder::new(&mut self.object).add_field(key, node)
    }

    pub fn entry_or_insert_nested(&mut self, key: impl Into<String>) -> ObjectMutBuilder<'_> {
        self.entry_or_insert(key, ObjectKind::Nested)
    }

    pub fn entry_or_insert_flattened(&mut self, key: impl Into<String>) -> ObjectMutBuilder<'_> {
        self.entry_or_insert(key, ObjectKind::Flattened)
    }

    pub fn entry_or_insert(&mut self, key: impl Into<String>, kind: ObjectKind) -> ObjectMutBuilder<'_> {
        let node = self
            .object
            .fields
            .entry(key.into())
            .or_insert(ResultNode::Object(Object::new(kind)));

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

    pub fn add_field(&mut self, key: impl Into<String>, node: ResultNode) {
        self.object.fields.insert(key.into(), node);
    }
}
