use crate::{
    ast::{self, Argument, FieldId, SchemaAst, TopId, WithIdentifier},
    diagnostics::{DatamodelError, Diagnostics},
};
use dml::scalars::ScalarType;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};

/// Resolved names for use in the validation process.
pub(crate) struct Names<'ast> {
    /// Models and enums
    tops: HashMap<&'ast str, TopId>,
    /// Generators have their own namespace.
    generators: HashMap<&'ast str, TopId>,
    /// Datasources have their own namespace.
    datasources: HashMap<&'ast str, TopId>,
    model_fields: BTreeMap<(TopId, &'ast str), FieldId>,
}

impl<'ast> Names<'ast> {
    pub(crate) fn new(ast: &'ast SchemaAst, diagnostics: &mut Diagnostics) -> Self {
        let mut names = Names {
            tops: Default::default(),
            generators: Default::default(),
            datasources: Default::default(),
            model_fields: Default::default(),
        };
        let mut tmp_names: HashSet<&str> = HashSet::new(); // throwaway container for duplicate checking

        for (top_id, top) in ast.iter_tops() {
            assert_is_not_a_reserved_scalar_type(top, diagnostics);

            let namespace = match top {
                ast::Top::Enum(ast_enum) => {
                    tmp_names.clear();

                    for value in &ast_enum.values {
                        if !tmp_names.insert(&value.name.name) {
                            diagnostics.push_error(DatamodelError::new_duplicate_enum_value_error(
                                &ast_enum.name.name,
                                &value.name.name,
                                value.span,
                            ))
                        }
                    }

                    &mut names.tops
                }
                ast::Top::Model(model) => {
                    for (field_id, field) in model.iter_fields() {
                        if names
                            .model_fields
                            .insert((top_id, &field.name.name), field_id)
                            .is_some()
                        {
                            diagnostics.push_error(DatamodelError::new_duplicate_field_error(
                                &model.name.name,
                                &field.name.name,
                                field.identifier().span,
                            ))
                        }
                    }

                    &mut names.tops
                }
                ast::Top::Source(datasource) => {
                    check_for_duplicate_properties(top, &datasource.properties, &mut tmp_names, diagnostics);
                    &mut names.datasources
                }
                ast::Top::Generator(generator) => {
                    check_for_duplicate_properties(top, &generator.properties, &mut tmp_names, diagnostics);
                    &mut names.generators
                }
                ast::Top::Type(_) => &mut names.tops,
            };

            insert_name(top_id, top, namespace, diagnostics, ast)
        }

        names
    }

    pub(crate) fn get_enum(&self, name: &str, schema: &'ast ast::SchemaAst) -> Option<&'ast ast::Enum> {
        self.tops.get(name).and_then(|top_id| schema[*top_id].as_enum())
    }
}

fn insert_name<'ast>(
    top_id: TopId,
    top: &'ast ast::Top,
    namespace: &mut HashMap<&'ast str, TopId>,
    diagnostics: &mut Diagnostics,
    schema: &'ast SchemaAst,
) {
    if let Some(existing) = namespace.insert(top.name(), top_id) {
        diagnostics.push_error(duplicate_top_error(&schema[existing], top));
    }
}

fn duplicate_top_error(existing: &ast::Top, duplicate: &ast::Top) -> DatamodelError {
    DatamodelError::new_duplicate_top_error(
        duplicate.name(),
        duplicate.get_type(),
        existing.get_type(),
        duplicate.identifier().span,
    )
}

fn assert_is_not_a_reserved_scalar_type(top: &ast::Top, diagnostics: &mut Diagnostics) {
    let ident = top.identifier();
    if ScalarType::from_str(&ident.name).is_ok() {
        diagnostics.push_error(DatamodelError::new_reserved_scalar_type_error(&ident.name, ident.span));
    }
}

fn check_for_duplicate_properties<'a>(
    top: &ast::Top,
    props: &'a [Argument],
    tmp_names: &mut HashSet<&'a str>,
    diagnostics: &mut Diagnostics,
) {
    tmp_names.clear();
    for arg in props {
        if !tmp_names.insert(&arg.name.name) {
            diagnostics.push_error(DatamodelError::new_duplicate_config_key_error(
                &format!("{} \"{}\"", top.get_type(), top.name()),
                &arg.name.name,
                arg.identifier().span,
            ));
        }
    }
}
