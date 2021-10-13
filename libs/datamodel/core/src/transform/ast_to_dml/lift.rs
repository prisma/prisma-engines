use crate::{ast, dml, transform::ast_to_dml::db};
use std::collections::HashMap;

/// Helper for lifting a datamodel.
///
/// When lifting, the AST is converted to the Datamodel data structure, and
/// additional semantics are attached.
pub(crate) struct LiftAstToDml<'a> {
    db: &'a db::ParserDatabase<'a>,
}

impl<'a> LiftAstToDml<'a> {
    pub(crate) fn new(db: &'a db::ParserDatabase<'a>) -> LiftAstToDml<'a> {
        LiftAstToDml { db }
    }

    pub(crate) fn lift(&self) -> dml::Datamodel {
        let mut schema = dml::Datamodel::new();

        for (top_id, ast_obj) in self.db.ast().iter_tops() {
            match (top_id, ast_obj) {
                (ast::TopId::Enum(id), ast::Top::Enum(en)) => schema.add_enum(self.lift_enum(id, en)),
                (ast::TopId::Model(model_id), ast::Top::Model(ty)) => schema.add_model(self.lift_model(model_id, ty)),
                (_, ast::Top::Source(_)) => { /* Source blocks are explicitly ignored by the validator */ }
                (_, ast::Top::Generator(_)) => { /* Generator blocks are explicitly ignored by the validator */ }
                (_, ast::Top::Type(_)) => { /* Type blocks are inlined */ }
                _ => unreachable!(),
            }
        }

        schema
    }

    /// Internal: Validates a model AST node and lifts it to a DML model.
    fn lift_model(&self, model_id: ast::ModelId, ast_model: &ast::Model) -> dml::Model {
        let mut model = dml::Model::new(ast_model.name.name.clone(), None);
        let model_attributes = self.db.walk_model(model_id);

        model.documentation = ast_model.documentation.clone().map(|comment| comment.text);
        model.database_name = model_attributes.attributes().mapped_name.map(String::from);
        model.is_ignored = model_attributes.attributes().is_ignored;

        model.primary_key = model_attributes.primary_key().map(|pk| dml::PrimaryKeyDefinition {
            name: pk.name().map(String::from),
            db_name: pk.final_database_name().map(|c| c.into_owned()),
            fields: pk.iter_ast_fields().map(|field| field.name.name.to_owned()).collect(),
            defined_on_field: pk.is_defined_on_field(),
        });

        model.indices = model_attributes
            .indexes()
            .map(|idx| dml::IndexDefinition {
                name: idx.attribute().name.map(String::from),
                db_name: Some(idx.final_database_name().into_owned()),
                fields: idx
                    .attribute()
                    .fields
                    .iter()
                    .map(|id| self.db.ast()[model_id][*id].name.name.clone())
                    .collect(),
                tpe: match idx.attribute().is_unique {
                    true => dml::IndexType::Unique,
                    false => dml::IndexType::Normal,
                },
                defined_on_field: idx.attribute().source_field.is_some(),
            })
            .collect();

        let active_connector = self.db.active_connector();

        // We iterate over scalar fields, then relation fields, but we want the
        // order of fields in the dml::Model to match the order of the fields in
        // the AST, so we need this bit of extra bookkeeping.
        let mut field_ids_for_sorting: HashMap<&str, ast::FieldId> = HashMap::with_capacity(ast_model.fields.len());

        for (field_id, scalar_field_data) in self.db.iter_model_scalar_fields(model_id) {
            let ast_field = &ast_model[field_id];
            let arity = self.lift_field_arity(&ast_field.arity);
            let field_type = self.lift_scalar_field_type(ast_field, &scalar_field_data.r#type, scalar_field_data);

            let mut field = dml::ScalarField::new(&ast_field.name.name, arity, field_type);

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.is_ignored = scalar_field_data.is_ignored;
            field.is_updated_at = scalar_field_data.is_updated_at;
            field.database_name = scalar_field_data.mapped_name.map(String::from);
            field.default_value = scalar_field_data.default.clone();

            field_ids_for_sorting.insert(&ast_field.name.name, field_id);
            model.add_field(dml::Field::ScalarField(field));
        }

        for relation_field in model_attributes.relation_fields() {
            let ast_field = relation_field.ast_field();
            let arity = self.lift_field_arity(&ast_field.arity);
            let attributes = relation_field.attributes();
            let target_model = &self.db.ast()[attributes.referenced_model];
            let relation_info = dml::RelationInfo::new(target_model.name());

            let mut field = dml::RelationField::new(&ast_field.name.name, arity, arity, relation_info);

            field.supports_restrict_action(
                active_connector.supports_referential_action(dml::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(active_connector.emulates_referential_actions());

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.relation_info.name = relation_field.relation_name().to_string();
            field.relation_info.on_delete = attributes.on_delete;
            field.relation_info.on_update = attributes.on_update;
            field.is_ignored = attributes.is_ignored;

            field.relation_info.references = attributes
                .references
                .as_ref()
                .map(|references| references.iter().map(|s| target_model[*s].name().to_owned()).collect())
                .unwrap_or_default();

            field.relation_info.fields = attributes
                .fields
                .as_ref()
                .map(|fields| {
                    fields
                        .iter()
                        .map(|id| self.db.ast()[model_id][*id].name.name.clone())
                        .collect()
                })
                .unwrap_or_default();

            field.relation_info.fk_name = relation_field.final_foreign_key_name().map(|cow| cow.into_owned());

            field_ids_for_sorting.insert(&ast_field.name.name, relation_field.field_id());
            model.add_field(dml::Field::RelationField(field))
        }

        model.fields.sort_by_key(|f| field_ids_for_sorting.get(f.name()));
        model
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&self, enum_id: ast::EnumId, ast_enum: &ast::Enum) -> dml::Enum {
        let mut en = dml::Enum::new(&ast_enum.name.name, vec![]);

        for (value_idx, ast_enum_value) in ast_enum.values.iter().enumerate() {
            en.add_value(self.lift_enum_value(ast_enum_value, enum_id, value_idx as u32));
        }

        en.documentation = ast_enum.documentation.clone().map(|comment| comment.text);
        en.database_name = self.db.get_enum_database_name(enum_id).map(String::from);
        en
    }

    /// Internal: Lifts an enum value AST node.
    fn lift_enum_value(&self, ast_value: &ast::EnumValue, enum_id: ast::EnumId, value_idx: u32) -> dml::EnumValue {
        let mut enum_value = dml::EnumValue::new(&ast_value.name.name);
        enum_value.documentation = ast_value.documentation.clone().map(|comment| comment.text);
        enum_value.database_name = self
            .db
            .get_enum_value_database_name(enum_id, value_idx)
            .map(String::from);

        enum_value
    }

    /// Internal: Lift a field's arity.
    fn lift_field_arity(&self, ast_field: &ast::FieldArity) -> dml::FieldArity {
        match ast_field {
            ast::FieldArity::Required => dml::FieldArity::Required,
            ast::FieldArity::Optional => dml::FieldArity::Optional,
            ast::FieldArity::List => dml::FieldArity::List,
        }
    }

    fn lift_scalar_field_type(
        &self,
        ast_field: &ast::Field,
        scalar_field_type: &db::ScalarFieldType,
        scalar_field_data: &db::ScalarField<'_>,
    ) -> dml::FieldType {
        match scalar_field_type {
            db::ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].name.name;
                dml::FieldType::Enum(enum_name.to_owned())
            }
            db::ScalarFieldType::Unsupported => {
                dml::FieldType::Unsupported(ast_field.field_type.as_unsupported().unwrap().0.to_owned())
            }
            db::ScalarFieldType::Alias(top_id) => {
                let alias = &self.db.ast()[*top_id];
                let scalar_field_type = self.db.alias_scalar_field_type(top_id);
                self.lift_scalar_field_type(alias, scalar_field_type, scalar_field_data)
            }
            db::ScalarFieldType::BuiltInScalar(scalar_type) => {
                let native_type = scalar_field_data.native_type.as_ref().map(|(name, args)| {
                    self.db
                        .active_connector()
                        .parse_native_type(name, args.clone())
                        .unwrap()
                });
                dml::FieldType::Scalar(scalar_type.to_owned(), None, native_type)
            }
        }
    }
}
