use crate::{EnumType, InputField, InputObjectType, InputType, ObjectType};
use std::ops;

/// Internal data structure for QuerySchema. It manages the normalized data about input, output
/// and enum types.
#[derive(Default, Debug)]
pub struct QuerySchemaDatabase {
    input_object_types: Vec<InputObjectType>,
    input_object_fields: Vec<InputField>,
    output_object_types: Vec<ObjectType>,
    enum_types: Vec<EnumType>,

    /// Possible types for input fields. This is an internal implementation detail, it should stay
    /// private.
    pub input_field_types: Vec<InputType>,
}

impl QuerySchemaDatabase {
    pub fn extend_input_fields(
        &mut self,
        input_object_id: InputObjectTypeId,
        fields: impl Iterator<Item = InputField>,
    ) {
        let mut object_type = &mut self.input_object_types[input_object_id.0];
        object_type.fields.0 = self.input_object_fields.len();
        for field in fields {
            object_type.fields.1 += 1;
            self.input_object_fields.push(field);
        }
    }

    pub fn find_input_object_field(&self, input_object_id: InputObjectTypeId, field_name: &str) -> Option<&InputField> {
        self.input_object_fields(input_object_id).find(|f| f.name == field_name)
    }

    pub fn input_object_fields(&self, input_object_id: InputObjectTypeId) -> impl Iterator<Item = &InputField> + Clone {
        let input_object = &self[input_object_id];
        let (start, len) = input_object.fields;
        self.input_object_fields[start..(start + len)].iter()
    }

    pub(crate) fn iter_enum_types(&self) -> impl Iterator<Item = &EnumType> {
        self.enum_types.iter()
    }

    pub fn push_input_field(&mut self, input_object_id: InputObjectTypeId, input_field: InputField) {
        let (start, len) = &mut self.input_object_types[input_object_id.0].fields;
        if *len == 0 {
            *start = self.input_object_fields.len();
        }
        *len += 1;
        self.input_object_fields.push(input_field);
    }

    pub fn push_input_object_type(&mut self, ty: InputObjectType) -> InputObjectTypeId {
        let id = InputObjectTypeId(self.input_object_types.len());
        self.input_object_types.push(ty);
        id
    }

    pub fn push_output_object_type(&mut self, ty: ObjectType) -> OutputObjectTypeId {
        let id = OutputObjectTypeId(self.output_object_types.len());
        self.output_object_types.push(ty);
        id
    }

    pub fn push_enum_type(&mut self, ty: EnumType) -> EnumTypeId {
        let id = EnumTypeId(self.enum_types.len());
        self.enum_types.push(ty);
        id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputObjectTypeId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputObjectTypeId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnumTypeId(usize);

pub type OutputFieldId = (OutputObjectTypeId, usize);

impl ops::Index<InputObjectTypeId> for QuerySchemaDatabase {
    type Output = InputObjectType;

    fn index(&self, index: InputObjectTypeId) -> &Self::Output {
        &self.input_object_types[index.0]
    }
}

impl ops::Index<OutputObjectTypeId> for QuerySchemaDatabase {
    type Output = ObjectType;

    fn index(&self, index: OutputObjectTypeId) -> &Self::Output {
        &self.output_object_types[index.0]
    }
}

impl ops::Index<EnumTypeId> for QuerySchemaDatabase {
    type Output = EnumType;

    fn index(&self, index: EnumTypeId) -> &Self::Output {
        &self.enum_types[index.0]
    }
}
