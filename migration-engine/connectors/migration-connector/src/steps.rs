use datamodel::*;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "stepType")]
pub enum MigrationStep {
    CreateModel(CreateModel),
    UpdateModel(UpdateModel),
    DeleteModel(DeleteModel),
    CreateField(CreateField),
    DeleteField(DeleteField),
    UpdateField(UpdateField),
    CreateEnum(CreateEnum),
    UpdateEnum(UpdateEnum),
    DeleteEnum(DeleteEnum),
    CreateIndex(CreateIndex),
    DeleteIndex(DeleteIndex),
}

pub trait WithDbName {
    fn db_name(&self) -> String;
}

// Deserializes the cases undefined, null and Some(T) into an Option<Option<T>>
fn some_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateModel {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,

    pub embedded: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateModel {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub db_name: Option<Option<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded: Option<bool>,
}

impl UpdateModel {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.embedded.is_some() || self.db_name.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteModel {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateField {
    pub model: String,

    pub name: String,

    #[serde(rename = "type")]
    pub tpe: FieldType,

    pub arity: FieldArity,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_created_at: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updated_at: Option<bool>,

    pub is_unique: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_list: Option<ScalarListStrategy>,
}

impl WithDbName for CreateField {
    fn db_name(&self) -> String {
        match self.db_name {
            Some(ref db_name) => db_name.clone(),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateField {
    pub model: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tpe: Option<FieldType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arity: Option<FieldArity>,

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub db_name: Option<Option<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_created_at: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updated_at: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unique: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub id_info: Option<Option<IdInfo>>, // fixme: change to behaviour

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub default: Option<Option<Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub scalar_list: Option<Option<ScalarListStrategy>>,
}

impl UpdateField {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some()
            || self.tpe.is_some()
            || self.arity.is_some()
            || self.db_name.is_some()
            || self.is_created_at.is_some()
            || self.is_updated_at.is_some()
            || self.is_unique.is_some()
            || self.id_info.is_some()
            || self.default.is_some()
            || self.scalar_list.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteField {
    pub model: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEnum {
    pub name: String,
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
}

impl WithDbName for CreateEnum {
    fn db_name(&self) -> String {
        match self.db_name {
            Some(ref db_name) => db_name.clone(),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateEnum {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    pub db_name: Option<Option<String>>,
}

impl UpdateEnum {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.values.is_some() || self.db_name.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteEnum {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateIndex {
    pub model: String,

    pub name: Option<String>,
    pub is_unique: bool,
    pub fields: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteIndex {
    pub model: String,
    pub name: Option<String>,
    pub is_unique: bool,
    pub fields: Vec<String>,
}

/// Convenience trait for migration steps on model indexes.
pub trait IndexStep {
    /// Does the step apply to the given IndexDefinition?
    ///
    /// This will only work if the index definition and the step's model match.
    fn applies_to_index(&self, index_definition: &IndexDefinition) -> bool;
}

impl IndexStep for CreateIndex {
    fn applies_to_index(&self, index_definition: &IndexDefinition) -> bool {
        self.name == index_definition.name
            && self.is_unique == index_definition.is_unique
            && self.fields == index_definition.fields
    }
}

impl IndexStep for DeleteIndex {
    fn applies_to_index(&self, index_definition: &IndexDefinition) -> bool {
        self.name == index_definition.name
            && self.is_unique == index_definition.is_unique
            && self.fields == index_definition.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_index_must_apply_to_the_right_indexes() {
        let definition = IndexDefinition {
            fields: vec!["testColumn".into()],
            is_unique: true,
            name: None,
        };
        let mut delete_index = DeleteIndex {
            model: "ignored".into(),
            fields: vec!["testColumn".into()],
            is_unique: true,
            name: None,
        };

        assert!(delete_index.applies_to_index(&definition));

        delete_index.is_unique = false;

        // is_unique does not match
        assert!(!delete_index.applies_to_index(&definition));

        delete_index.is_unique = true;
        delete_index.name = Some("index_on_testColumn".to_owned());

        // name does not match
        assert!(!delete_index.applies_to_index(&definition));
    }

    #[test]
    fn create_index_must_apply_to_the_right_indexes() {
        let definition = IndexDefinition {
            fields: vec!["testColumn".into()],
            is_unique: true,
            name: None,
        };
        let mut create_index = CreateIndex {
            model: "ignored".into(),
            fields: vec!["testColumn".into()],
            is_unique: true,
            name: None,
        };

        assert!(create_index.applies_to_index(&definition));

        create_index.is_unique = false;

        // is_unique does not match
        assert!(!create_index.applies_to_index(&definition));

        create_index.is_unique = true;
        create_index.name = Some("index_on_testColumn".to_owned());

        // name does not match
        assert!(!create_index.applies_to_index(&definition));
    }
}
