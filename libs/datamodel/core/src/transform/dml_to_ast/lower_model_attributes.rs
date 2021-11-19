use ::dml::model::IndexAlgorithm;

use crate::ast::{Argument, Attribute};
use crate::common::constraint_names::ConstraintNames;
use crate::common::preview_features::PreviewFeature;
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Span},
    dml, Ignorable, IndexDefinition, IndexType, Model, SortOrder, WithDatabaseName,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a model's attributes.
    pub(crate) fn lower_model_attributes(&self, model: &dml::Model) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @@id

        if let Some(pk) = &model.primary_key {
            if !pk.defined_on_field {
                let mut args = if self.preview_features.contains(PreviewFeature::ExtendedIndexes) {
                    vec![ast::Argument::new_array("", LowerDmlToAst::pk_field_array(&pk.fields))]
                } else {
                    vec![ast::Argument::new_array(
                        "",
                        LowerDmlToAst::field_array(&pk.fields.clone().into_iter().map(|f| f.name).collect::<Vec<_>>()),
                    )]
                };

                if pk.name.is_some() {
                    args.push(ast::Argument::new(
                        "name",
                        ast::Expression::StringValue(String::from(pk.name.as_ref().unwrap()), Span::empty()),
                    ));
                }

                if pk.db_name.is_some() {
                    if let Some(src) = self.datasource {
                        if !ConstraintNames::primary_key_name_matches(pk, model, &*src.active_connector) {
                            args.push(ast::Argument::new(
                                "map",
                                ast::Expression::StringValue(String::from(pk.db_name.as_ref().unwrap()), Span::empty()),
                            ));
                        }
                    }
                }

                attributes.push(ast::Attribute::new("id", args));
            }
        }

        // @@unique
        model
            .indices
            .iter()
            .filter(|index| index.is_unique() && !index.defined_on_field)
            .for_each(|index_def| {
                let mut args = self.fields_argument(&index_def);
                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", name.to_string()));
                }

                self.push_index_map_argument(model, index_def, &mut args);

                attributes.push(ast::Attribute::new("unique", args));
            });

        // @@index
        model
            .indices
            .iter()
            .filter(|index| index.tpe == IndexType::Normal)
            .for_each(|index_def| {
                let mut args = self.fields_argument(&index_def);
                self.push_index_map_argument(model, index_def, &mut args);

                if let Some(IndexAlgorithm::Hash) = index_def.algorithm {
                    args.push(ast::Argument::new(
                        "type",
                        ast::Expression::ConstantValue("Hash".to_string(), Span::empty()),
                    ));
                };

                attributes.push(ast::Attribute::new("index", args));
            });

        // @@fulltext
        model
            .indices
            .iter()
            .filter(|index| index.is_fulltext())
            .for_each(|index_def| {
                let mut args = self.fields_argument(&index_def);
                self.push_index_map_argument(model, index_def, &mut args);

                attributes.push(ast::Attribute::new("fulltext", args));
            });

        // @@map
        <LowerDmlToAst<'a>>::push_model_index_map_arg(model, &mut attributes);

        // @@ignore
        if model.is_ignored() {
            attributes.push(ast::Attribute::new("ignore", vec![]));
        }

        attributes
    }

    fn fields_argument(&self, index_def: &&IndexDefinition) -> Vec<Argument> {
        if self.preview_features.contains(PreviewFeature::ExtendedIndexes) {
            vec![ast::Argument::new_array(
                "",
                LowerDmlToAst::index_field_array(&index_def.fields),
            )]
        } else {
            vec![ast::Argument::new_array(
                "",
                LowerDmlToAst::field_array(&index_def.fields.clone().into_iter().map(|f| f.name).collect::<Vec<_>>()),
            )]
        }
    }

    pub(crate) fn push_field_index_arguments(
        &self,
        model: &Model,
        index_def: &IndexDefinition,
        args: &mut Vec<Argument>,
    ) {
        let field = index_def.fields.first().unwrap();

        if let Some(src) = self.datasource {
            if !ConstraintNames::index_name_matches(index_def, model, &*src.active_connector) {
                args.push(ast::Argument::new(
                    "map",
                    ast::Expression::StringValue(String::from(index_def.db_name.as_ref().unwrap()), Span::empty()),
                ));
            }
            if self.preview_features.contains(PreviewFeature::ExtendedIndexes) {
                if let Some(length) = field.length {
                    args.push(ast::Argument::new(
                        "length",
                        ast::Expression::NumericValue(length.to_string(), Span::empty()),
                    ));
                }

                if field.sort_order == Some(SortOrder::Desc) {
                    args.push(ast::Argument::new(
                        "sort",
                        ast::Expression::ConstantValue("Desc".to_string(), Span::empty()),
                    ));
                }
            }
        }
    }

    pub(crate) fn push_index_map_argument(&self, model: &Model, index_def: &IndexDefinition, args: &mut Vec<Argument>) {
        if let Some(src) = self.datasource {
            if !ConstraintNames::index_name_matches(index_def, model, &*src.active_connector) {
                args.push(ast::Argument::new(
                    "map",
                    ast::Expression::StringValue(String::from(index_def.db_name.as_ref().unwrap()), Span::empty()),
                ));
            }
        }
    }

    pub(crate) fn push_model_index_map_arg<T: WithDatabaseName>(obj: &T, attributes: &mut Vec<Attribute>) {
        if let Some(db_name) = obj.database_name() {
            attributes.push(ast::Attribute::new(
                "map",
                vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                    String::from(db_name),
                    Span::empty(),
                ))],
            ));
        }
    }
}
