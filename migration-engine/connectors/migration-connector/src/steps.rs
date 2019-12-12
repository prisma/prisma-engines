//! Datamodel migration steps.

use datamodel::ast;
use serde::{Deserialize, Serialize};

/// An atomic change to a [Datamodel AST](datamodel/ast/struct.Datamodel.html).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", deny_unknown_fields)]
pub enum MigrationStep {
    CreateModel(CreateModel),
    UpdateModel(UpdateModel),
    DeleteModel(DeleteModel),
    CreateDirective(CreateDirective),
    DeleteDirective(DeleteDirective),
    CreateArgument(CreateArgument),
    UpdateArgument(UpdateArgument),
    DeleteArgument(DeleteArgument),
    CreateField(CreateField),
    DeleteField(DeleteField),
    UpdateField(UpdateField),
    CreateEnum(CreateEnum),
    UpdateEnum(UpdateEnum),
    DeleteEnum(DeleteEnum),
    CreateTypeAlias(CreateTypeAlias),
    UpdateTypeAlias(UpdateTypeAlias),
    DeleteTypeAlias(DeleteTypeAlias),
    CreateSource(CreateSource),
    DeleteSource(DeleteSource),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateModel {
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteModel {
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateField {
    pub model: String,

    pub field: String,

    #[serde(rename = "type")]
    pub tpe: String,

    pub arity: ast::FieldArity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteField {
    pub model: String,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEnum {
    pub r#enum: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteEnum {
    pub r#enum: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDirective {
    pub location: DirectiveLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteDirective {
    pub location: DirectiveLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", deny_unknown_fields)]
pub enum ArgumentLocation {
    Directive(DirectiveLocation),
    Source(SourceLocation),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectiveLocation {
    pub path: DirectivePath,
    pub directive: String,
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

    pub fn into_argument_location(self) -> ArgumentLocation {
        ArgumentLocation::Directive(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLocation {
    pub source: String,
}

impl SourceLocation {
    pub fn into_argument_location(self) -> ArgumentLocation {
        ArgumentLocation::Source(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", deny_unknown_fields)]
pub enum DirectivePath {
    Field {
        model: String,
        field: String,
    },
    Model {
        model: String,
    },
    Enum {
        r#enum: String,
    },
    TypeAlias {
        #[serde(rename = "typeAlias")]
        type_alias: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateArgument {
    pub location: ArgumentLocation,
    // TODO: figure out whether we want this, or an option, for default arguments
    pub argument: String,
    pub value: MigrationExpression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteArgument {
    pub location: ArgumentLocation,
    // TODO: figure out whether we want this, or an option, for default arguments
    pub argument: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateArgument {
    pub location: ArgumentLocation,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTypeAlias {
    pub type_alias: String,

    pub r#type: String,
    pub arity: ast::FieldArity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteTypeAlias {
    pub type_alias: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSource {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSource {
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn directive_location_serialization_gives_expected_json_shape() {
        let create_directive = CreateDirective {
            location: DirectiveLocation {
                path: DirectivePath::Field {
                    model: "Cat".to_owned(),
                    field: "owner".to_owned(),
                },
                directive: "status".to_owned(),
                arguments: None,
            },
        };

        let serialized_step = serde_json::to_value(&create_directive).unwrap();

        let expected_json = json!({
            "location": {
                "path": {
                    "tag": "Field",
                    "model": "Cat",
                    "field": "owner",
                },
                "directive": "status"
            }
        });

        println!("{}\n{}", serialized_step, expected_json);

        assert_eq!(serialized_step, expected_json);

        let deserialized_step: CreateDirective = serde_json::from_value(expected_json).unwrap();
        assert_eq!(create_directive, deserialized_step);
    }
}
