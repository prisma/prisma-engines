use crate::common::constraint_names::ConstraintNames;
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::PreviewFeature::NamedConstraints;
use crate::{
    ast::{self, Span},
    dml, Ignorable, IndexType, WithDatabaseName,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a model's attributes.
    pub(crate) fn lower_model_attributes(&self, model: &dml::Model) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @@id

        if let Some(pk) = &model.primary_key {
            if !pk.defined_on_field {
                let args = vec![ast::Argument::new_array("", LowerDmlToAst::field_array(&pk.fields))];

                attributes.push(ast::Attribute::new("id", args));
            }
        }

        // @@unique
        model
            .indices
            .iter()
            .filter(|index| index.is_unique() && !index.defined_on_field)
            .for_each(|index_def| {
                let mut args = vec![ast::Argument::new_array(
                    "",
                    LowerDmlToAst::field_array(&index_def.fields),
                )];

                if self.preview_features.contains(NamedConstraints) {
                    if let Some(name) = &index_def.name {
                        args.push(ast::Argument::new_string("name", name));
                    }

                    if let Some(src) = self.datasource {
                        if !ConstraintNames::index_name_matches(index_def, model, &*src.active_connector) {
                            args.push(ast::Argument::new(
                                "map",
                                ast::Expression::StringValue(
                                    String::from(index_def.db_name.as_ref().unwrap()),
                                    Span::empty(),
                                ),
                            ));
                        }
                    }
                } else {
                    if let Some(name) = &index_def.name {
                        args.push(ast::Argument::new_string("name", name));
                    }
                }

                attributes.push(ast::Attribute::new("unique", args));
            });

        // @@index
        model
            .indices
            .iter()
            .filter(|index| index.tpe == IndexType::Normal)
            .for_each(|index_def| {
                let mut args = vec![ast::Argument::new_array(
                    "",
                    LowerDmlToAst::field_array(&index_def.fields),
                )];

                if self.preview_features.contains(NamedConstraints) {
                    if let Some(src) = self.datasource {
                        if !ConstraintNames::index_name_matches(index_def, model, &*src.active_connector) {
                            args.push(ast::Argument::new(
                                "map",
                                ast::Expression::StringValue(
                                    String::from(index_def.db_name.as_ref().unwrap()),
                                    Span::empty(),
                                ),
                            ));
                        }
                    }
                } else {
                    if let Some(name) = &index_def.name {
                        args.push(ast::Argument::new_string("name", name));
                    }
                }

                attributes.push(ast::Attribute::new("index", args));
            });

        // @@map
        if let Some(db_name) = model.database_name() {
            attributes.push(ast::Attribute::new(
                "map",
                vec![ast::Argument::new_unnamed(ast::Expression::StringValue(
                    String::from(db_name),
                    Span::empty(),
                ))],
            ));
        }

        // @@ignore
        if model.is_ignored() {
            attributes.push(ast::Attribute::new("ignore", vec![]));
        }

        attributes
    }
}
