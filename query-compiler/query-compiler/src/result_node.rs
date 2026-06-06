use std::borrow::Cow;

use indexmap::IndexMap;
use query_structure::{FieldTypeInformation, TypeIdentifier};
use serde::{Serialize, Serializer, ser::SerializeMap, ser::SerializeStruct, ser::SerializeTuple};

use crate::{data_mapper::FieldType, expression::EnumsMap};

#[derive(Debug)]
pub enum ResultNode {
    AffectedRows,
    Object(Object),
    Field {
        db_name: Cow<'static, str>,
        field_type: FieldType,
    },
}

impl Serialize for ResultNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::AffectedRows => {
                let mut state = serializer.serialize_struct("ResultNode", 1)?;
                state.serialize_field("type", "affectedRows")?;
                state.end()
            }
            Self::Object(object) => object.serialize(serializer),
            Self::Field { db_name, field_type } => {
                let mut state = serializer.serialize_struct("ResultNode", 3)?;
                state.serialize_field("type", "field")?;
                state.serialize_field("dbName", db_name)?;
                state.serialize_field("fieldType", field_type)?;
                state.end()
            }
        }
    }
}

#[derive(Debug)]
pub struct Object {
    serialized_name: Option<Cow<'static, str>>,
    fields: IndexMap<Cow<'static, str>, ResultNode>,
    skip_nulls: bool,
}

impl Serialize for Object {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !self.skip_nulls {
            let mut tuple = serializer.serialize_tuple(2)?;
            tuple.serialize_element(&self.serialized_name)?;
            tuple.serialize_element(&SerializedObjectFields(&self.fields))?;
            return tuple.end();
        }

        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.serialized_name)?;
        tuple.serialize_element(&SerializedObjectFields(&self.fields))?;
        tuple.serialize_element(&self.skip_nulls)?;
        tuple.end()
    }
}

struct SerializedObjectFields<'a>(&'a IndexMap<Cow<'static, str>, ResultNode>);

impl Serialize for SerializedObjectFields<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_fields(self.0, serializer)
    }
}

fn serialize_fields<S>(fields: &IndexMap<Cow<'static, str>, ResultNode>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(fields.len()))?;
    for (name, node) in fields {
        match node {
            ResultNode::Field { db_name, field_type } if db_name == name => {
                if let Some(compact_name) = field_type.compact_name() {
                    map.serialize_entry(name, compact_name)?;
                } else {
                    map.serialize_entry(
                        name,
                        &FieldNodeInObject {
                            db_name: None,
                            field_type,
                        },
                    )?;
                }
            }
            ResultNode::Field { db_name, field_type } => {
                map.serialize_entry(
                    name,
                    &FieldNodeInObject {
                        db_name: Some(db_name),
                        field_type,
                    },
                )?;
            }
            node => map.serialize_entry(name, node)?,
        }
    }
    map.end()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldNodeInObject<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    db_name: Option<&'a Cow<'static, str>>,
    field_type: &'a FieldType,
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

    fn new_value_inner(&mut self, db_name: Cow<'static, str>, type_info: FieldTypeInformation) -> ResultNode {
        let field_type = FieldType::from(&type_info);
        if let TypeIdentifier::Enum(id) = type_info.typ.id {
            self.enums.add(type_info.typ.dm.zip(id));
        }
        ResultNode::Field { db_name, field_type }
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
