use crate::ast::{Argument, Attribute};
use crate::common::constraint_names::ConstraintNames;
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Span},
    dml, Ignorable, IndexDefinition, IndexType, Model, WithDatabaseName,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a model's attributes.
    pub(crate) fn lower_model_attributes(&self, model: &dml::Model) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @@id

        if let Some(pk) = &model.primary_key {
            if !pk.defined_on_field {
                let mut args = vec![ast::Argument::new_array("", LowerDmlToAst::field_array(&pk.fields))];

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
                let mut args = vec![ast::Argument::new_array(
                    "",
                    LowerDmlToAst::field_array(&index_def.fields),
                )];

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
                let mut args = vec![ast::Argument::new_array(
                    "",
                    LowerDmlToAst::field_array(&index_def.fields),
                )];

                self.push_index_map_argument(model, index_def, &mut args);

                attributes.push(ast::Attribute::new("index", args));
            });

        // @@map
        <LowerDmlToAst<'a>>::push_map_attribute(model, &mut attributes);

        // @@ignore
        if model.is_ignored() {
            attributes.push(ast::Attribute::new("ignore", vec![]));
        }

        attributes
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

    pub(crate) fn push_map_attribute<T: WithDatabaseName>(obj: &T, attributes: &mut Vec<Attribute>) {
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
