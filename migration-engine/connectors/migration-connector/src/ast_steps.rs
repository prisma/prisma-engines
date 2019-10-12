//! Datamodel migration steps.

use datamodel::ast;
use serde::{Deserialize, Deserializer, Serialize};

/// An atomic change to a [Datamodel AST](datamodel/ast/struct.Datamodel.html).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "stepType")]
pub enum MigrationStep {
    CreateModel(CreateModel),
    UpdateModel(UpdateModel),
    DeleteModel(DeleteModel),
    CreateDirective(CreateDirective),
    UpdateDirective(UpdateDirective),
    DeleteDirective(DeleteDirective),
    CreateField(CreateField),
    DeleteField(DeleteField),
    UpdateField(UpdateField),
    CreateEnum(CreateEnum),
    UpdateEnum(UpdateEnum),
    DeleteEnum(DeleteEnum),
}

pub trait WithDbName {
    fn db_name(&self) -> String;
}

/// Deserializes the cases `undefined`, `null` and `Some(T)` into an `Option<Option<T>>`.
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateField {
    pub model: String,

    pub name: String,

    #[serde(rename = "type")]
    // #[serde(serialize_with = "serialize_identifier")]
    // #[serde(deserialize_with = "deserialize_identifier")]
    pub tpe: String,

    pub arity: ast::FieldArity,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub is_created_at: Option<bool>,

    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub is_updated_at: Option<bool>,

    // pub is_unique: bool,

    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub id: Option<IdInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub scalar_list: Option<ScalarListStrategy>,
}

impl WithDbName for CreateField {
    fn db_name(&self) -> String {
        match self.db_name {
            Some(ref db_name) => db_name.clone(),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateField {
    pub model: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tpe: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arity: Option<ast::FieldArity>,
    // #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    // pub db_name: Option<Option<String>>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub is_created_at: Option<bool>,

    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub is_updated_at: Option<bool>,

    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub is_unique: Option<bool>,
    // #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    // pub id_info: Option<Option<IdInfo>>, // fixme: change to behaviour

    // #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    // pub default: Option<Option<ast::Value>>,
    // #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "some_option")]
    // pub scalar_list: Option<Option<ScalarListStrategy>>,
}

impl UpdateField {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.tpe.is_some() || self.arity.is_some()
        // || self.db_name.is_some()
        // || self.is_created_at.is_some()
        // || self.is_updated_at.is_some()
        // || self.is_unique.is_some()
        // || self.id_info.is_some()
        // || self.default.is_some()
        // || self.scalar_list.is_some()
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
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateEnum {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_values: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_values: Option<Vec<String>>,
}

impl UpdateEnum {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.created_values.is_some() || self.deleted_values.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteEnum {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirective {
    #[serde(flatten)]
    pub locator: DirectiveLocator,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDirective {
    #[serde(flatten)]
    pub locator: DirectiveLocator,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDirective {
    #[serde(flatten)]
    pub locator: DirectiveLocator,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveLocator {
    #[serde(flatten)]
    pub location: DirectiveLocation,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields, untagged)]
pub enum DirectiveLocation {
    Field { model: String, field: String },
    Model { model: String },
    Enum { r#enum: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn directive_location_serialization_gives_expected_json_shape() {
        let create_directive = CreateDirective {
            locator: DirectiveLocator {
                location: DirectiveLocation::Field {
                    model: "Cat".to_owned(),
                    field: "owner".to_owned(),
                },
                name: "status".to_owned(),
            },
        };

        let serialized_step = serde_json::to_value(&create_directive).unwrap();
        let expected_json = json!({
            "model": "Cat",
            "field": "owner",
            "name": "status",
        });

        assert_eq!(serialized_step, expected_json);

        let deserialized_step: CreateDirective = serde_json::from_value(expected_json).unwrap();
        assert_eq!(create_directive, deserialized_step);
    }
}
