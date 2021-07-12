use crate::transform::dml_to_ast::LowerDmlToAst;
use crate::{
    ast::{self, Span},
    dml, Ignorable, IndexType, WithDatabaseName,
};

impl<'a> LowerDmlToAst<'a> {
    /// Internal: Lowers a model's attributes.
    pub(crate) fn lower_model_attributes(&self, model: &dml::Model) -> Vec<ast::Attribute> {
        let mut attributes = vec![];

        // @@id
        if !model.id_fields.is_empty() {
            let args = vec![ast::Argument::new_array(
                "",
                LowerDmlToAst::field_array(&model.id_fields),
            )];

            attributes.push(ast::Attribute::new("id", args));
        }

        // @@unique
        model
            .indices
            .iter()
            .filter(|index| index.tpe == IndexType::Unique)
            .for_each(|index_def| {
                let mut args = vec![ast::Argument::new_array(
                    "",
                    LowerDmlToAst::field_array(&index_def.fields),
                )];

                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", name));
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

                if let Some(name) = &index_def.name {
                    args.push(ast::Argument::new_string("name", name));
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
