//! Datamodel migration steps.

use datamodel::ast;
use serde::{Deserialize, Serialize};

/// An atomic change to a [Datamodel AST](datamodel/ast/struct.Datamodel.html).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "stepType")]
pub enum MigrationStep {
    CreateModel(CreateModel),
    UpdateModel(UpdateModel),
    DeleteModel(DeleteModel),
    CreateDirective(CreateDirective),
    DeleteDirective(DeleteDirective),
    CreateDirectiveArgument(CreateDirectiveArgument),
    UpdateDirectiveArgument(UpdateDirectiveArgument),
    DeleteDirectiveArgument(DeleteDirectiveArgument),
    CreateField(CreateField),
    DeleteField(DeleteField),
    UpdateField(UpdateField),
    CreateEnum(CreateEnum),
    UpdateEnum(UpdateEnum),
    DeleteEnum(DeleteEnum),
    CreateTypeAlias(CreateTypeAlias),
    UpdateTypeAlias(UpdateTypeAlias),
    DeleteTypeAlias(DeleteTypeAlias),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateModel {
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateModel {
    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
}

impl UpdateModel {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteModel {
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateField {
    pub model: String,

    pub field: String,

    #[serde(rename = "type")]
    pub tpe: String,

    pub arity: ast::FieldArity,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateField {
    pub model: String,

    pub field: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tpe: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arity: Option<ast::FieldArity>,
}

impl UpdateField {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.tpe.is_some() || self.arity.is_some()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteField {
    pub model: String,
    pub field: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEnum {
    pub r#enum: String,
    pub values: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateEnum {
    pub r#enum: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub created_values: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub deleted_values: Vec<String>,
}

impl UpdateEnum {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.created_values.len() > 0 || self.deleted_values.len() > 0
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteEnum {
    pub r#enum: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirective {
    #[serde(flatten)]
    pub locator: DirectiveLocation,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDirective {
    #[serde(flatten)]
    pub locator: DirectiveLocation,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Argument {
    pub name: String,
    pub value: MigrationExpression,
}

impl Argument {
    fn matches_ast_argument(&self, argument: &ast::Argument) -> bool {
        self.name == argument.name.name && self.value == MigrationExpression::from_ast_expression(&argument.value)
    }
}

impl From<&ast::Argument> for Argument {
    fn from(arg: &ast::Argument) -> Self {
        Argument {
            name: arg.name.name.clone(),
            value: MigrationExpression::from_ast_expression(&arg.value),
        }
    }
}

impl Into<ast::Argument> for &Argument {
    fn into(self) -> ast::Argument {
        ast::Argument {
            name: ast::Identifier::new(&self.name),
            value: self.value.to_ast_expression(),
            span: ast::Span::empty(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveLocation {
    #[serde(flatten)]
    pub location: DirectiveType,
    pub directive: String,
    /// The arguments of the directive are required to match directives that can be repeated,
    /// like `@@unique` on a model. This is `None` when matching can be done without comparing
    /// the arguments, and `Some` when a directive should be matched exactly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Argument>>,
}

impl DirectiveLocation {
    pub fn matches_ast_directive(&self, directive: &ast::Directive) -> bool {
        if self.directive != directive.name.name {
            return false;
        }

        match &self.arguments {
            Some(arguments) => {
                if directive.arguments.len() != arguments.len() {
                    return false;
                }

                directive.arguments.iter().all(|directive_argument| {
                    arguments
                        .iter()
                        .find(|self_argument| self_argument.matches_ast_argument(directive_argument))
                        .is_some()
                })
            }
            None => true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields, untagged)]
pub enum DirectiveType {
    TypeAlias { type_alias: String },
    Field { model: String, field: String },
    Model { model: String },
    Enum { r#enum: String },
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectiveArgument {
    #[serde(flatten)]
    pub directive_location: DirectiveLocation,
    // TODO: figure out whether we want this, or an option, for default arguments
    pub argument: String,
    pub value: MigrationExpression,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDirectiveArgument {
    #[serde(flatten)]
    pub directive_location: DirectiveLocation,
    // TODO: figure out whether we want this, or an option, for default arguments
    pub argument: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDirectiveArgument {
    #[serde(flatten)]
    pub directive_location: DirectiveLocation,
    pub argument: String,
    // TODO: figure out whether we want this, or an option, for default arguments
    pub new_value: MigrationExpression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationExpression(pub String);

impl MigrationExpression {
    pub fn to_ast_expression(&self) -> ast::Expression {
        self.0.parse().unwrap()
    }

    pub fn from_ast_expression(expr: &ast::Expression) -> Self {
        MigrationExpression(expr.render_to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTypeAlias {
    pub type_alias: String,

    pub r#type: String,
    pub arity: ast::FieldArity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateTypeAlias {
    pub type_alias: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

impl UpdateTypeAlias {
    pub fn is_any_option_set(&self) -> bool {
        self.r#type.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteTypeAlias {
    pub type_alias: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn directive_location_serialization_gives_expected_json_shape() {
        let create_directive = CreateDirective {
            locator: DirectiveLocation {
                location: DirectiveType::Field {
                    model: "Cat".to_owned(),
                    field: "owner".to_owned(),
                },
                directive: "status".to_owned(),
                arguments: None,
            },
        };

        let serialized_step = serde_json::to_value(&create_directive).unwrap();
        let expected_json = json!({
            "model": "Cat",
            "field": "owner",
            "directive": "status",
        });

        assert_eq!(serialized_step, expected_json);

        let deserialized_step: CreateDirective = serde_json::from_value(expected_json).unwrap();
        assert_eq!(create_directive, deserialized_step);
    }
}
