use super::{
    field_builders::RelationFieldBuilder, relation_builder::RelationBuilder, CompositeTypeBuilder, FieldBuilder,
    IndexBuilder, ModelBuilder, PrimaryKeyBuilder,
};
use crate::{
    builders::{CompositeFieldBuilder, ScalarFieldBuilder},
    extensions::*,
    IndexType, InlineRelation, RelationLinkManifestation, RelationSide, RelationTable, TypeIdentifier,
};
use dml::{self, CompositeTypeFieldType, Datamodel, Ignorable, WithDatabaseName};
use psl::{datamodel_connector::RelationMode, schema_ast::ast};

pub(crate) fn model_builders(
    datamodel: &Datamodel,
    relation_placeholders: &[RelationPlaceholder],
) -> Vec<ModelBuilder> {
    datamodel
        .models()
        .filter(|model| !model.is_ignored())
        .filter(|model| model.is_supported())
        .map(|model| ModelBuilder {
            id: model.id,
            name: model.name.clone(),
            fields: model_field_builders(model, relation_placeholders),
            manifestation: model.database_name().map(|s| s.to_owned()),
            primary_key: pk_builder(model),
            indexes: index_builders(model),
            supports_create_operation: model.supports_create_operation(),
            dml_model: model.clone(),
        })
        .collect()
}

fn model_field_builders(model: &dml::Model, relations: &[RelationPlaceholder]) -> Vec<FieldBuilder> {
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
                if !relation.model_a.is_relation_supported(&relation.field_a)
                    || !relation.model_b.is_relation_supported(&relation.field_b)
                {
                    return None;
                }

                Some(FieldBuilder::Relation(RelationFieldBuilder {
                    name: rf.name.clone(),
                    arity: rf.arity,
                    relation_name: relation.name.clone(),
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

pub(crate) fn relation_builders(
    placeholders: &[RelationPlaceholder],
    relation_mode: RelationMode,
) -> Vec<RelationBuilder> {
    placeholders
        .iter()
        .filter(|r| r.model_a.is_relation_supported(&r.field_a) && r.model_b.is_relation_supported(&r.field_b))
        .map(|r| RelationBuilder {
            name: r.name(),
            manifestation: r.manifestation(),
            model_a_id: r.model_a.id,
            model_b_id: r.model_b.id,
            relation_mode,
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
pub(crate) fn relation_placeholders(datamodel: &dml::Datamodel) -> Vec<RelationPlaceholder> {
    let mut result = Vec::new();

    for model in datamodel.models().filter(|model| !model.is_ignored) {
        for field in model.relation_fields().filter(|field| !field.is_ignored) {
            let dml::RelationInfo {
                referenced_model: to,
                references,
                name,
                ..
            } = &field.relation_info;

            let related_model = datamodel.find_model_by_id(*to).unwrap();

            let (_, related_field) = datamodel.find_related_field_bang(field);
            let related_field_info: &dml::RelationInfo = &related_field.relation_info;

            let (model_a, model_b, field_a, field_b, referenced_fields_a, referenced_fields_b) = match () {
                _ if model.name < related_model.name => (
                    model.clone(),
                    related_model.clone(),
                    field.clone(),
                    related_field.clone(),
                    references,
                    &related_field_info.references,
                ),
                _ if related_model.name < model.name => (
                    related_model.clone(),
                    model.clone(),
                    related_field.clone(),
                    field.clone(),
                    &related_field_info.references,
                    references,
                ),
                // Self-relation case
                _ => {
                    let (field_a, field_b) = if field.name < related_field.name {
                        (field.clone(), related_field.clone())
                    } else {
                        (related_field.clone(), field.clone())
                    };
                    (
                        model.clone(),
                        related_model.clone(),
                        field_a,
                        field_b,
                        references,
                        &related_field_info.references,
                    )
                }
            };

            let inline_on_model_a = ManifestationPlaceholder::Inline {
                in_table_of_model: model_a.id,
                field: field_a.clone(),
                referenced_fields: referenced_fields_a.clone(),
            };

            let inline_on_model_b = ManifestationPlaceholder::Inline {
                in_table_of_model: model_b.id,
                field: field_b.clone(),
                referenced_fields: referenced_fields_b.clone(),
            };

            let inline_on_this_model = ManifestationPlaceholder::Inline {
                in_table_of_model: model.id,
                field: field.clone(),
                referenced_fields: references.clone(),
            };

            let inline_on_related_model = ManifestationPlaceholder::Inline {
                in_table_of_model: related_model.id,
                field: related_field.clone(),
                referenced_fields: related_field_info.references.clone(),
            };

            let manifestation = match (field_a.is_list(), field_b.is_list()) {
                (true, true) => ManifestationPlaceholder::Table,
                (false, true) => inline_on_model_a,
                (true, false) => inline_on_model_b,
                (false, false) => match (references.first(), &related_field_info.references.first()) {
                    (Some(_), None) => inline_on_this_model,
                    (None, Some(_)) => inline_on_related_model,
                    (None, None) => {
                        if model_a.name < model_b.name {
                            inline_on_model_a
                        } else {
                            inline_on_model_b
                        }
                    }
                    (Some(_), Some(_)) => {
                        panic!("It's not allowed that both sides of a relation specify the inline policy. The field was {} on model {}. The related field was {} on model {}.", field.name, model.name, related_field.name, related_model.name)
                    }
                },
            };

            let placeholder = RelationPlaceholder {
                name: name.clone(),
                model_a,
                model_b,
                field_a,
                field_b,
                manifestation,
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
pub struct RelationPlaceholder {
    pub name: String,
    pub model_a: dml::Model,
    pub model_b: dml::Model,
    pub field_a: dml::RelationField,
    pub field_b: dml::RelationField,
    pub manifestation: ManifestationPlaceholder,
}

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Debug, Clone)]
pub enum ManifestationPlaceholder {
    Inline {
        in_table_of_model: ast::ModelId,
        /// The relation field.
        field: dml::RelationField,
        /// The name of the (dml) fields referenced by the relation.
        referenced_fields: Vec<String>,
    },
    Table,
}

#[allow(unused)]
impl RelationPlaceholder {
    fn name(&self) -> String {
        // TODO: must replicate behaviour of `generateRelationName` from `SchemaInferrer`
        match &self.name as &str {
            "" => format!("{}To{}", &self.model_a.name, &self.model_b.name),
            _ => self.name.clone(),
        }
    }

    pub fn table_name(&self) -> String {
        format!("_{}", self.name())
    }

    pub fn model_a_column(&self) -> String {
        "A".to_string()
    }

    pub fn model_b_column(&self) -> String {
        "B".to_string()
    }

    pub fn is_one_to_one(&self) -> bool {
        !self.field_a.is_list() && !self.field_b.is_list()
    }

    fn is_many_to_many(&self) -> bool {
        self.field_a.is_list() && self.field_b.is_list()
    }

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

    fn manifestation(&self) -> RelationLinkManifestation {
        match &self.manifestation {
            // TODO: relation table columns must get renamed: lowercased type names instead of A and B
            ManifestationPlaceholder::Table => RelationLinkManifestation::RelationTable(RelationTable {
                table: self.table_name(),
                model_a_column: self.model_a_column(),
                model_b_column: self.model_b_column(),
            }),
            ManifestationPlaceholder::Inline { in_table_of_model, .. } => {
                RelationLinkManifestation::Inline(InlineRelation {
                    in_table_of_model: *in_table_of_model,
                })
            }
        }
    }
}
