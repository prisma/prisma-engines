use crate::ast::{Argument, Attribute};
use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Span},
    dml::{self, Ignorable, IndexDefinition, IndexType, Model, SortOrder, WithDatabaseName},
};
use ::dml::datamodel::Datamodel;
use ::dml::model::IndexAlgorithm;

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a model's attributes.
    pub(crate) fn lower_model_attributes(&self, datamodel: &Datamodel, model: &dml::Model) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @@id

        if let Some(pk) = &model.primary_key {
            if !pk.defined_on_field {
                let mut args = vec![ast::Argument::new_unnamed(ast::Expression::Array(
                    LowerDmlToAst::pk_field_array(&pk.fields),
                    ast::Span::empty(),
                ))];

                if pk.name.is_some() {
                    args.push(ast::Argument::new(
                        "name",
                        ast::Expression::StringValue(String::from(pk.name.as_ref().unwrap()), Span::empty()),
                    ));
                }

                if pk.db_name.is_some() {
                    if let Some(src) = self.datasource {
                        if !super::primary_key_name_matches(pk, model, &*src.active_connector) {
                            args.push(ast::Argument::new(
                                "map",
                                ast::Expression::StringValue(String::from(pk.db_name.as_ref().unwrap()), Span::empty()),
                            ));
                        }
                    }
                }

                if matches!(pk.clustered, Some(false)) {
                    args.push(ast::Argument::new(
                        "clustered",
                        ast::Expression::ConstantValue("false".to_string(), Span::empty()),
                    ));
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
                let mut args = self.fields_argument(index_def, false);
                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", name.to_string()));
                }

                self.push_index_map_argument(datamodel, model, index_def, &mut args);

                if matches!(index_def.clustered, Some(true)) {
                    args.push(ast::Argument::new(
                        "clustered",
                        ast::Expression::NumericValue("true".to_string(), Span::empty()),
                    ));
                }

                attributes.push(ast::Attribute::new("unique", args));
            });

        // @@index
        model
            .indices
            .iter()
            .filter(|index| index.tpe == IndexType::Normal)
            .for_each(|index_def| {
                let mut args = self.fields_argument(index_def, false);
                self.push_index_map_argument(datamodel, model, index_def, &mut args);

                match index_def.algorithm {
                    Some(IndexAlgorithm::BTree) | None => (),
                    Some(algo) => {
                        args.push(ast::Argument::new(
                            "type",
                            ast::Expression::ConstantValue(algo.to_string(), Span::empty()),
                        ));
                    }
                }

                if matches!(index_def.clustered, Some(true)) {
                    args.push(ast::Argument::new(
                        "clustered",
                        ast::Expression::ConstantValue("true".to_string(), Span::empty()),
                    ));
                }

                attributes.push(ast::Attribute::new("index", args));
            });

        // @@fulltext
        model
            .indices
            .iter()
            .filter(|index| index.is_fulltext())
            .for_each(|index_def| {
                let mut args = self.fields_argument(index_def, true);
                self.push_index_map_argument(datamodel, model, index_def, &mut args);

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

    fn fields_argument(&self, index_def: &IndexDefinition, always_render_sort_order: bool) -> Vec<Argument> {
        vec![ast::Argument::new_unnamed(ast::Expression::Array(
            LowerDmlToAst::index_field_array(&index_def.fields, always_render_sort_order),
            ast::Span::empty(),
        ))]
    }

    pub(crate) fn push_field_index_arguments(
        &self,
        datamodel: &Datamodel,
        model: &Model,
        index_def: &IndexDefinition,
        args: &mut Vec<Argument>,
    ) {
        let field = index_def.fields.first().unwrap();

        if let Some(src) = self.datasource {
            if !super::index_name_matches(index_def, datamodel, model, &*src.active_connector) {
                args.push(ast::Argument::new(
                    "map",
                    ast::Expression::StringValue(String::from(index_def.db_name.as_ref().unwrap()), Span::empty()),
                ));
            }

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

    pub(crate) fn push_index_map_argument(
        &self,
        datamodel: &Datamodel,
        model: &Model,
        index_def: &IndexDefinition,
        args: &mut Vec<Argument>,
    ) {
        if let Some(src) = self.datasource {
            if !super::index_name_matches(index_def, datamodel, model, &*src.active_connector) {
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
