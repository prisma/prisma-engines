mod datasource_serializer;
mod generator_serializer;
mod lower;
mod lower_enum_attributes;
mod lower_enum_value_attributes;
mod lower_field;
mod lower_model_attributes;

pub use datasource_serializer::add_sources_to_ast;
pub use generator_serializer::GeneratorSerializer;
pub use lower::LowerDmlToAst;

use crate::{ast, configuration::StringFromEnvVar};
use ::dml::{model::*, traits::*};
use datamodel_connector::{constraint_names::ConstraintNames, Connector};

fn lower_string_from_env_var(arg_name: &str, string_from_env: &StringFromEnvVar) -> ast::ConfigBlockProperty {
    match string_from_env.as_env_var() {
        Some(ref env_var) => {
            let values = ast::ArgumentsList {
                arguments: vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                    env_var.to_string(),
                    ast::Span::empty(),
                ))],
                ..Default::default()
            };
            ast::ConfigBlockProperty {
                name: ast::Identifier::new(arg_name),
                value: ast::Expression::Function("env".to_owned(), values, ast::Span::empty()),
                span: ast::Span::empty(),
            }
        }
        None => ast::ConfigBlockProperty {
            name: ast::Identifier::new(arg_name),
            value: ast::Expression::StringValue(string_from_env.as_literal().unwrap().to_string(), ast::Span::empty()),
            span: ast::Span::empty(),
        },
    }
}

fn primary_key_name_matches(pk: &PrimaryKeyDefinition, model: &Model, connector: &dyn Connector) -> bool {
    pk.db_name.as_ref().unwrap() == &ConstraintNames::primary_key_name(model.final_database_name(), connector)
}

pub fn foreign_key_name_matches(
    ri: &::dml::relation_info::RelationInfo,
    model: &Model,
    connector: &dyn Connector,
) -> bool {
    let column_names: Vec<&str> = ri
        .fields
        .iter()
        .map(|field_name| {
            // We cannot unwrap here, due to us re-introspecting relations
            // and if we're not using foreign keys, we might copy a relation
            // that is not valid anymore. We still want to write that to the
            // file and let user fix it, but if we unwrap here, we will
            // panic.
            model
                .find_scalar_field(field_name)
                .map(|field| field.final_database_name())
                .unwrap_or(field_name)
        })
        .collect();

    ri.fk_name.as_ref().unwrap()
        == &ConstraintNames::foreign_key_constraint_name(model.final_database_name(), &column_names, connector)
}

pub fn index_name_matches(idx: &IndexDefinition, model: &Model, connector: &dyn Connector) -> bool {
    let column_names: Vec<&str> = idx
        .fields
        .iter()
        .map(|field| {
            model
                .find_scalar_field(&field.path.first().unwrap().0)
                .unwrap()
                .final_database_name()
        })
        .collect();

    let expected = if idx.is_unique() {
        ConstraintNames::unique_index_name(model.final_database_name(), &column_names, connector)
    } else {
        ConstraintNames::non_unique_index_name(model.final_database_name(), &column_names, connector)
    };

    idx.db_name.as_deref().unwrap() == expected
}
