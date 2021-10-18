use crate::{
    ast, dml,
    transform::ast_to_dml::db::{self, walkers::*},
};
use ::dml::composite_type::{CompositeType, CompositeTypeField, CompositeTypeFieldType};
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

        // We iterate over scalar fields, then relations, but we want the
        // order of fields in the dml::Model to match the order of the fields in
        // the AST, so we need this bit of extra bookkeeping.
        //
        // (model_idx, field_name) -> sort_key
        let mut field_ids_for_sorting: HashMap<(&str, &str), ast::FieldId> = HashMap::new();

        for (top_id, ast_obj) in self.db.ast().iter_tops() {
            match (top_id, ast_obj) {
                (ast::TopId::Enum(id), ast::Top::Enum(en)) => schema.add_enum(self.lift_enum(id, en)),
                (ast::TopId::Model(model_id), ast::Top::Model(ty)) => {
                    schema.add_model(self.lift_model(model_id, ty, &mut field_ids_for_sorting))
                }
                (ast::TopId::CompositeType(ct_id), ast::Top::CompositeType(_)) => {
                    schema.composite_types.push(self.lift_composite_type(ct_id))
                }
                (_, ast::Top::Source(_)) => { /* Source blocks are explicitly ignored by the validator */ }
                (_, ast::Top::Generator(_)) => { /* Generator blocks are explicitly ignored by the validator */ }
                (_, ast::Top::Type(_)) => { /* Type blocks are inlined */ }
                _ => unreachable!(),
            }
        }

        self.lift_relations(&mut schema, &mut field_ids_for_sorting);

        for model in schema.models_mut() {
            let model_name = model.name.as_str();
            model
                .fields
                .sort_by_key(|field| field_ids_for_sorting[&(model_name, field.name())]);
        }

        schema
    }

    fn lift_relations(
        &self,
        schema: &mut dml::Datamodel,
        field_ids_for_sorting: &mut HashMap<(&'a str, &'a str), ast::FieldId>,
    ) {
        let active_connector = self.db.active_connector();
        let common_dml_fields = |field: &mut dml::RelationField,
                                 attributes: &super::db::RelationField<'_>,
                                 relation_field: RelationFieldWalker<'_, '_>| {
            let ast_field = relation_field.ast_field();
            field.relation_info.on_delete = attributes.on_delete;
            field.relation_info.on_update = attributes.on_update;
            field.relation_info.name = relation_field.relation_name().to_string();
            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.is_ignored = attributes.is_ignored;
            field.relation_info.fk_name = relation_field.final_foreign_key_name().map(|cow| cow.into_owned());
            field.supports_restrict_action(
                active_connector.supports_referential_action(dml::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(active_connector.emulates_referential_actions());
        };

        for relation in self.db.walk_relations() {
            match relation.refine() {
                RefinedRelationWalker::Inline(relation) => {
                    // Forward field
                    if let Some(relation_field) = relation.forward_relation_field() {
                        let attributes = relation_field.attributes();
                        let arity = self.lift_field_arity(&relation_field.ast_field().arity);
                        let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = self.lift_field_arity(&relation_field.referential_arity());
                        let mut field =
                            dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                        common_dml_fields(&mut field, attributes, relation_field);

                        field.relation_info.references = attributes
                            .references
                            .as_ref()
                            .map(|references| {
                                references
                                    .iter()
                                    .map(|s| relation_field.related_model().scalar_field(*s).name().to_owned())
                                    .collect()
                            })
                            .unwrap_or_default();

                        field.relation_info.fields = attributes
                            .fields
                            .as_ref()
                            .map(|fields| {
                                fields
                                    .iter()
                                    .map(|id| relation_field.model().scalar_field(*id).name().to_owned())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let model = schema.find_model_mut(relation_field.model().name());
                        model.add_field(dml::Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }

                    // Back field
                    if let Some(relation_field) = relation.back_relation_field() {
                        let ast_field = relation_field.ast_field();
                        let attributes = relation_field.attributes();
                        let arity = self.lift_field_arity(&ast_field.arity);
                        let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = self.lift_field_arity(&relation_field.referential_arity());
                        let mut field =
                            dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                        common_dml_fields(&mut field, attributes, relation_field);

                        // TODO: delete this block.
                        // This block is required because of validations that happen after lifting, but these will be moved to ParserDatabase.
                        // If you can delete this and the tests pass, it safe to delete it.
                        {
                            field.relation_info.references = attributes
                                .references
                                .as_ref()
                                .map(|references| {
                                    references
                                        .iter()
                                        .map(|s| relation_field.related_model().scalar_field(*s).name().to_owned())
                                        .collect()
                                })
                                .unwrap_or_default();

                            field.relation_info.fields = attributes
                                .fields
                                .as_ref()
                                .map(|fields| {
                                    fields
                                        .iter()
                                        .map(|id| relation_field.model().scalar_field(*id).name().to_owned())
                                        .collect()
                                })
                                .unwrap_or_default();
                        }

                        let model = schema.find_model_mut(relation_field.model().name());
                        model.add_field(dml::Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }
                }
                RefinedRelationWalker::ImplicitManyToMany(relation) => {
                    for relation_field in [relation.field_a(), relation.field_b()] {
                        let ast_field = relation_field.ast_field();
                        let attributes = relation_field.attributes();
                        let arity = self.lift_field_arity(&ast_field.arity);
                        let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = self.lift_field_arity(&relation_field.referential_arity());
                        let mut field =
                            dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                        common_dml_fields(&mut field, attributes, relation_field);

                        // The entire conditional can be replaced with just unwrapping the primary key, once all the required validations happen inside ParserDatabase.
                        // If you can simplify this block and the tests pass, it means that's the case.
                        if attributes.references.is_some() {
                            field.relation_info.references = attributes
                                .references
                                .as_ref()
                                .map(|references| {
                                    references
                                        .iter()
                                        .map(|s| relation_field.related_model().scalar_field(*s).name().to_owned())
                                        .collect()
                                })
                                .unwrap_or_default();
                        } else {
                            // We can unwrap here once we validate the presence of an id field in the parser database. If you can unwrap
                            // and the tests pass, do it.
                            let primary_key = match relation_field.related_model().primary_key() {
                                Some(pk) => pk,
                                _ => continue, // Let this be handled as one of the leftover fields
                            };

                            field.relation_info.references =
                                primary_key.fields().map(|field| field.name().to_owned()).collect();
                        };

                        let model = schema.find_model_mut(relation_field.model().name());
                        model.add_field(dml::Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }
                }
            }
        }

        // TEMPORARY until this is validated in ParserDatabase. If you delete this and the test suite passes, it can be deleted.
        for relation_field in self.db.walk_models().flat_map(|m| m.relation_fields()) {
            // This is the load-bearing `if`. We re-add all relation fields to the datamodel that weren't inferred as part
            //  of a relation, so further validations can happen.
            //
            //  The whole loop can be deleted once these validations happen before lifting.
            if field_ids_for_sorting.contains_key(&(relation_field.model().name(), relation_field.name())) {
                continue;
            }

            let ast_field = relation_field.ast_field();
            let attributes = relation_field.attributes();
            let arity = self.lift_field_arity(&ast_field.arity);
            let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
            let referential_arity = self.lift_field_arity(&relation_field.referential_arity());
            let mut field = dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

            common_dml_fields(&mut field, attributes, relation_field);

            field.relation_info.references = attributes
                .references
                .as_ref()
                .map(|references| {
                    references
                        .iter()
                        .map(|s| relation_field.related_model().scalar_field(*s).name().to_owned())
                        .collect()
                })
                .unwrap_or_default();

            field.relation_info.fields = attributes
                .fields
                .as_ref()
                .map(|fields| {
                    fields
                        .iter()
                        .map(|id| relation_field.model().scalar_field(*id).name().to_owned())
                        .collect()
                })
                .unwrap_or_default();

            let model = schema.find_model_mut(relation_field.model().name());
            model.add_field(dml::Field::RelationField(field));
            field_ids_for_sorting.insert(
                (relation_field.model().name(), relation_field.name()),
                relation_field.field_id(),
            );
        }
    }

    fn lift_composite_type(&self, ct_id: ast::CompositeTypeId) -> CompositeType {
        let mut fields = Vec::new();
        let walker = self.db.walk_composite_type(ct_id);

        for field in walker.fields() {
            let field = CompositeTypeField {
                name: field.name().to_owned(),
                r#type: self.lift_composite_type_field_type(field.r#type()),
                arity: self.lift_field_arity(&field.arity()),
                database_name: field.mapped_name().map(String::from),
                documentation: field.documentation().map(ToString::to_string),
            };

            fields.push(field);
        }

        CompositeType {
            name: walker.name().to_owned(),
            fields,
        }
    }

    /// Internal: Validates a model AST node and lifts it to a DML model.
    fn lift_model(
        &self,
        model_id: ast::ModelId,
        ast_model: &'a ast::Model,
        field_ids_for_sorting: &mut HashMap<(&'a str, &'a str), ast::FieldId>,
    ) -> dml::Model {
        let mut model = dml::Model::new(ast_model.name.name.clone(), None);
        let walker = self.db.walk_model(model_id);

        model.documentation = ast_model.documentation.clone().map(|comment| comment.text);
        model.database_name = walker.attributes().mapped_name.map(String::from);
        model.is_ignored = walker.attributes().is_ignored;

        model.primary_key = walker.primary_key().map(|pk| dml::PrimaryKeyDefinition {
            name: pk.name().map(String::from),
            db_name: pk.final_database_name().map(|c| c.into_owned()),
            fields: pk.iter_ast_fields().map(|field| field.name.name.to_owned()).collect(),
            defined_on_field: pk.is_defined_on_field(),
        });

        model.indices = walker
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

        for scalar_field in walker.scalar_fields() {
            let field_id = scalar_field.field_id();
            let attributes = scalar_field.attributes();
            let ast_field = &ast_model[field_id];
            let arity = self.lift_field_arity(&ast_field.arity);
            let field_type = match &attributes.r#type {
                db::ScalarFieldType::CompositeType(ctid) => {
                    let mut field = dml::CompositeField::new();
                    field.composite_type = self.db.ast()[*ctid].name.name.to_owned();
                    field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
                    field.is_ignored = attributes.is_ignored;
                    field.database_name = attributes.mapped_name.map(String::from);

                    model.add_field(dml::Field::CompositeField(field));
                    continue;
                }
                _ => self.lift_scalar_field_type(ast_field, &attributes.r#type, attributes),
            };

            let mut field = dml::ScalarField::new(&ast_field.name.name, arity, field_type);

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.is_ignored = attributes.is_ignored;
            field.is_updated_at = attributes.is_updated_at;
            field.database_name = attributes.mapped_name.map(String::from);
            field.default_value = attributes.default.clone();

            field_ids_for_sorting.insert((&ast_model.name.name, &ast_field.name.name), field_id);
            model.add_field(dml::Field::ScalarField(field));
        }

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
    fn lift_field_arity(&self, field_arity: &ast::FieldArity) -> dml::FieldArity {
        match field_arity {
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
            db::ScalarFieldType::CompositeType(_) => {
                unreachable!();
            }
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

    fn lift_composite_type_field_type(&self, scalar_field_type: &db::ScalarFieldType) -> CompositeTypeFieldType {
        match scalar_field_type {
            db::ScalarFieldType::CompositeType(ctid) => {
                CompositeTypeFieldType::CompositeType(self.db.ast()[*ctid].name.name.to_owned())
            }
            db::ScalarFieldType::BuiltInScalar(scalar_type) => {
                CompositeTypeFieldType::Scalar(scalar_type.to_owned(), None, None)
            }
            db::ScalarFieldType::Alias(_) | db::ScalarFieldType::Enum(_) | db::ScalarFieldType::Unsupported => {
                unreachable!()
            }
        }
    }
}
