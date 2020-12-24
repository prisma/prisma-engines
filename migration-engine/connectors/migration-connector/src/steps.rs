//! Datamodel migration steps.

use datamodel::ast;

/// An atomic change to a [Datamodel AST](datamodel/ast/struct.Datamodel.html).
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct CreateModel {
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct UpdateModel {
    pub model: String,
    pub new_name: Option<String>,
}

impl UpdateModel {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteModel {
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct CreateField {
    pub model: String,
    pub field: String,
    pub tpe: String,
    pub arity: FieldArity,
}

#[derive(Debug, Clone)]
pub struct UpdateField {
    pub model: String,
    pub field: String,
    pub new_name: Option<String>,
    pub tpe: Option<String>,
    pub arity: Option<FieldArity>,
}

impl UpdateField {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || self.tpe.is_some() || self.arity.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteField {
    pub model: String,
    pub field: String,
}

#[derive(Debug, Clone)]
pub struct CreateEnum {
    pub r#enum: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateEnum {
    pub r#enum: String,
    pub new_name: Option<String>,
    pub created_values: Vec<String>,
    pub deleted_values: Vec<String>,
}

impl UpdateEnum {
    pub fn is_any_option_set(&self) -> bool {
        self.new_name.is_some() || !self.created_values.is_empty() || !self.deleted_values.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteEnum {
    pub r#enum: String,
}

#[derive(Debug, Clone)]
pub struct CreateDirective {
    pub location: DirectiveLocation,
}

#[derive(Debug, Clone)]
pub struct DeleteDirective {
    pub location: DirectiveLocation,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub enum ArgumentLocation {
    Directive(DirectiveLocation),
    Source(SourceLocation),
}

#[derive(Debug, Clone)]
pub struct DirectiveLocation {
    pub path: DirectivePath,
    pub directive: String,
}

impl DirectiveLocation {
    pub fn matches_ast_directive(&self, directive: &ast::Attribute) -> bool {
        if self.directive != directive.name.name {
            return false;
        }
        match &self.path {
            DirectivePath::Model {
                model: _,
                arguments: Some(arguments),
            } => {
                if directive.arguments.len() != arguments.len() {
                    return false;
                }

                directive.arguments.iter().all(|directive_argument| {
                    arguments
                        .iter()
                        .any(|self_argument| self_argument.matches_ast_argument(directive_argument))
                })
            }
            _ => true,
        }
    }

    pub fn into_argument_location(self) -> ArgumentLocation {
        ArgumentLocation::Directive(self)
    }
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub source: String,
}

impl SourceLocation {
    pub fn into_argument_location(self) -> ArgumentLocation {
        ArgumentLocation::Source(self)
    }
}

#[derive(Debug, Clone)]
pub enum DirectivePath {
    Field {
        model: String,
        field: String,
    },
    Model {
        model: String,
        arguments: Option<Vec<Argument>>,
    },
    Enum {
        r#enum: String,
    },
    EnumValue {
        r#enum: String,
        value: String,
    },
    TypeAlias {
        type_alias: String,
    },
}

impl DirectivePath {
    pub fn set_arguments(self, arguments: Vec<Argument>) -> Self {
        match self {
            Self::Model { model, arguments: _ } => Self::Model {
                model,
                arguments: Some(arguments),
            },
            _ => self,
        }
    }

    pub fn arguments(&self) -> &Option<Vec<Argument>> {
        match &self {
            Self::Model { model: _, arguments } => &arguments,
            _ => &None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateArgument {
    pub location: ArgumentLocation,
    pub argument: String,
    pub value: MigrationExpression,
}

#[derive(Debug, Clone)]
pub struct DeleteArgument {
    pub location: ArgumentLocation,
    pub argument: String,
}

#[derive(Debug, Clone)]
pub struct UpdateArgument {
    pub location: ArgumentLocation,
    pub argument: String,
    pub new_value: MigrationExpression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationExpression(pub String);

impl MigrationExpression {
    pub fn to_ast_expression(&self) -> ast::Expression {
        self.0.parse().unwrap()
    }

    pub fn from_ast_expression(expr: &ast::Expression) -> Self {
        MigrationExpression(expr.render_to_string())
    }
}

#[derive(Debug, Clone)]
pub struct CreateTypeAlias {
    pub type_alias: String,
    pub r#type: String,
    pub arity: FieldArity,
}

#[derive(Debug, Clone)]
pub struct UpdateTypeAlias {
    pub type_alias: String,
    pub r#type: Option<String>,
}

impl UpdateTypeAlias {
    pub fn is_any_option_set(&self) -> bool {
        self.r#type.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteTypeAlias {
    pub type_alias: String,
}

#[derive(Debug, Clone)]
pub struct CreateSource {
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct DeleteSource {
    pub source: String,
}

#[derive(Debug, Copy, Clone)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

impl From<ast::FieldArity> for FieldArity {
    fn from(ast_arity: ast::FieldArity) -> Self {
        (&ast_arity).into()
    }
}

impl From<&ast::FieldArity> for FieldArity {
    fn from(ast_arity: &ast::FieldArity) -> Self {
        match &ast_arity {
            ast::FieldArity::Required => FieldArity::Required,
            ast::FieldArity::Optional => FieldArity::Optional,
            ast::FieldArity::List => FieldArity::List,
        }
    }
}

impl Into<ast::FieldArity> for FieldArity {
    fn into(self) -> ast::FieldArity {
        (&self).into()
    }
}

impl Into<ast::FieldArity> for &FieldArity {
    fn into(self) -> ast::FieldArity {
        match &self {
            FieldArity::Required => ast::FieldArity::Required,
            FieldArity::Optional => ast::FieldArity::Optional,
            FieldArity::List => ast::FieldArity::List,
        }
    }
}
