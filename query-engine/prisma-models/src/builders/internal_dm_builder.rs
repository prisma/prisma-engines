use super::{
    field_builders::RelationFieldBuilder, CompositeTypeBuilder, FieldBuilder, IndexBuilder, ModelBuilder,
    PrimaryKeyBuilder,
};
use crate::{
    builders::{CompositeFieldBuilder, ScalarFieldBuilder},
    extensions::*,
    IndexType, RelationSide, TypeIdentifier,
};
use dml::{self, CompositeTypeFieldType, Datamodel, Ignorable, WithDatabaseName};

pub(crate) fn model_builders(
    datamodel: &Datamodel,
    relation_placeholders: &[RelationPlaceholder],
    schema: &psl::ValidatedSchema,
) -> Vec<ModelBuilder> {
    datamodel
        .models()
        .filter(|model| !model.is_ignored())
        .filter(|model| model.is_supported())
        .map(|model| ModelBuilder {
            id: model.id,
            name: model.name.clone(),
            fields: model_field_builders(model, relation_placeholders, schema),
            manifestation: model.database_name().map(|s| s.to_owned()),
            primary_key: pk_builder(model),
            indexes: index_builders(model),
            supports_create_operation: model.supports_create_operation(),
            dml_model: model.clone(),
        })
        .collect()
}

fn model_field_builders(
    model: &dml::Model,
    relations: &[RelationPlaceholder],
    schema: &psl::ValidatedSchema,
) -> Vec<FieldBuilder> {
    model
        .fields()
        .filter(|field| !field.is_ignored())
        .filter_map(|field| match field {
            dml::Field::CompositeField(cf) => Some(FieldBuilder::Composite(CompositeFieldBuilder {
                name: cf.name.clone(),
                db_name: cf.database_name.clone(),
                arity: cf.arity,
                type_name: cf.composite_type.clone(),
                default_value: cf.default_value.clone(),
            })),
            dml::Field::RelationField(rf) => {
                let relation = relations
                    .iter()
                    .find(|r| r.is_for_model_and_field(model, rf))
                    .unwrap_or_else(|| {
                        panic!("Did not find a relation for model {} and field {}", model.name, rf.name)
                    });
                // If one side of the relation is not supported, filter out the relation
                if schema.db.walk(relation.id).is_ignored() {
                    return None;
                }

                Some(FieldBuilder::Relation(RelationFieldBuilder {
                    id: rf.id,
                    name: rf.name.clone(),
                    arity: rf.arity,
                    relation_name: schema.db.walk(relation.id).relation_name().to_string(),
                    relation_side: relation.relation_side(rf),
                    relation_info: rf.relation_info.clone(),
                    on_delete_default: rf.default_on_delete_action(),
                    on_update_default: rf.default_on_update_action(),
                }))
            }
            dml::Field::ScalarField(sf) => {
                if sf.type_identifier() == TypeIdentifier::Unsupported {
                    None
                } else {
                    Some(FieldBuilder::Scalar(ScalarFieldBuilder {
                        name: sf.name.clone(),
                        type_identifier: sf.type_identifier(),
                        is_unique: model.field_is_unique(&sf.name),
                        is_id: model.field_is_primary(&sf.name),
                        is_auto_generated_int_id: model.field_is_auto_generated_int_id(&sf.name),
                        is_autoincrement: sf.is_auto_increment(),
                        is_updated_at: sf.is_updated_at,
                        internal_enum: sf.field_type.as_enum(),
                        arity: sf.arity,
                        db_name: sf.database_name.clone(),
                        default_value: sf.default_value.clone(),
                        native_type: sf.native_type(),
                    }))
                }
            }
        })
        .collect()
}

fn composite_field_builders(composite: &dml::CompositeType) -> Vec<FieldBuilder> {
    composite
        .fields
        .iter()
        // .filter(|field| !field.is_ignored()) // Todo(?): Composites are not ignorable at the moment.
        .filter_map(|field| match &field.r#type {
            CompositeTypeFieldType::CompositeType(type_name) => Some(FieldBuilder::Composite(CompositeFieldBuilder {
                name: field.name.clone(),
                db_name: field.database_name.clone(),
                arity: field.arity,
                type_name: type_name.clone(),
                // No defaults on composite fields of type composite
                default_value: None,
            })),
            CompositeTypeFieldType::Scalar(_, _) | CompositeTypeFieldType::Enum(_) => {
                let type_ident = field.type_identifier();

                if type_ident == TypeIdentifier::Unsupported {
                    None
                } else {
                    Some(FieldBuilder::Scalar(ScalarFieldBuilder {
                        name: field.name.clone(),
                        type_identifier: type_ident,
                        is_unique: false, // Composites can't have uniques or ids at the moment.
                        is_id: false,
                        is_auto_generated_int_id: false,
                        is_autoincrement: false,
                        is_updated_at: false, // Todo: This info isn't available here.
                        internal_enum: field.r#type.as_enum(),
                        arity: field.arity,
                        db_name: field.database_name.clone(),
                        default_value: field.default_value.as_ref().cloned(),
                        native_type: field.native_type(),
                    }))
                }
            }
            CompositeTypeFieldType::Unsupported(_) => None,
        })
        .collect()
}

fn index_builders(model: &dml::Model) -> Vec<IndexBuilder> {
    model
        .indices
        .iter()
        .filter(|i| i.fields.len() > 1 && model.is_compound_index_supported(i)) // @@unique for 1 field are transformed to is_unique instead
        .filter(|i| i.fields.iter().all(|f| f.path.len() <= 1)) // TODO: we do not take indices with composite fields for now
        .map(|i| IndexBuilder {
            name: i.name.clone(),
            fields: i
                .fields
                .clone()
                .into_iter()
                .map(|mut f| f.path.pop().unwrap().0)
                .collect(),
            typ: match i.tpe {
                dml::IndexType::Unique => IndexType::Unique,
                dml::IndexType::Normal => IndexType::Normal,
                // TODO: When introducing the indexes in QE, change this.
                dml::IndexType::Fulltext => IndexType::Normal,
            },
        })
        .collect()
}

fn pk_builder(model: &dml::Model) -> Option<PrimaryKeyBuilder> {
    model.primary_key.as_ref().map(|pk| PrimaryKeyBuilder {
        fields: pk.fields.clone().into_iter().map(|f| f.name).collect(),
        alias: pk.name.to_owned(),
    })
}

pub(crate) fn composite_type_builders(datamodel: &Datamodel) -> Vec<CompositeTypeBuilder> {
    datamodel
        .composite_types
        .iter()
        .map(|ct| CompositeTypeBuilder {
            name: ct.name.clone(),
            fields: composite_field_builders(ct),
        })
        .collect()
}

/// Calculates placeholders that are used to compute builders dependent on some relation information being present already.
pub(crate) fn relation_placeholders(
    datamodel: &dml::Datamodel,
    schema: &psl::ValidatedSchema,
) -> Vec<RelationPlaceholder> {
    let mut result = Vec::new();

    for model in datamodel.models().filter(|model| !model.is_ignored) {
        for field in model.relation_fields().filter(|field| !field.is_ignored) {
            let (model_id, field_id) = field.id.coarsen();
            let relation_field = schema.db.walk(model_id).relation_field(field_id);
            let relation_id = relation_field.relation().id;
            let dml::RelationInfo {
                referenced_model: to, ..
            } = &field.relation_info;

            let related_model = datamodel.find_model_by_id(*to).unwrap();

            let (_, related_field) = datamodel.find_related_field_bang(field);

            let (model_a, model_b, field_a, field_b) = match () {
                _ if model.name < related_model.name => (
                    model.clone(),
                    related_model.clone(),
                    field.clone(),
                    related_field.clone(),
                ),
                _ if related_model.name < model.name => (
                    related_model.clone(),
                    model.clone(),
                    related_field.clone(),
                    field.clone(),
                ),
                // Self-relation case
                _ => {
                    let (field_a, field_b) = if field.name < related_field.name {
                        (field.clone(), related_field.clone())
                    } else {
                        (related_field.clone(), field.clone())
                    };
                    (model.clone(), related_model.clone(), field_a, field_b)
                }
            };

            let placeholder = RelationPlaceholder {
                id: relation_id,
                model_a,
                model_b,
                field_a,
                field_b,
            };

            // Skip duplicate placeholders
            if !result.contains(&placeholder) {
                result.push(placeholder);
            }
        }
    }

    result
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RelationPlaceholder {
    pub id: psl::parser_database::RelationId,
    pub model_a: dml::Model,
    pub model_b: dml::Model,
    pub field_a: dml::RelationField,
    pub field_b: dml::RelationField,
}

impl RelationPlaceholder {
    fn is_for_model_and_field(&self, model: &dml::Model, field: &dml::RelationField) -> bool {
        (&self.model_a == model && &self.field_a == field) || (&self.model_b == model && &self.field_b == field)
    }

    fn relation_side(&self, field: &dml::RelationField) -> RelationSide {
        if field == &self.field_a {
            RelationSide::A
        } else if field == &self.field_b {
            RelationSide::B
        } else {
            panic!("this field is not part of the relations")
        }
    }
}
