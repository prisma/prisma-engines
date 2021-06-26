use crate::*;
use datamodel::{dml, DefaultValue, Ignorable, NativeTypeInstance, WithDatabaseName};
use itertools::Itertools;

pub struct DatamodelConverter<'a> {
    datamodel: &'a dml::Datamodel,
    relations: Vec<TempRelationHolder>,
}

impl<'a> DatamodelConverter<'a> {
    pub fn convert_string(datamodel: String) -> InternalDataModelTemplate {
        let datamodel = datamodel::parse_datamodel(&datamodel).unwrap().subject;
        Self::convert(&datamodel)
    }

    pub fn convert(datamodel: &dml::Datamodel) -> InternalDataModelTemplate {
        DatamodelConverter::new(datamodel).convert_internal()
    }

    fn new(datamodel: &dml::Datamodel) -> DatamodelConverter {
        DatamodelConverter {
            datamodel,
            relations: Self::calculate_relations(datamodel),
        }
    }

    fn convert_internal(&self) -> InternalDataModelTemplate {
        InternalDataModelTemplate {
            models: self.convert_models(),
            relations: self.convert_relations(),
            enums: self.convert_enums(),
            version: Some("v2".to_string()),
        }
    }

    fn convert_enums(&self) -> Vec<InternalEnum> {
        self.datamodel
            .enums()
            .map(|e| InternalEnum {
                name: e.name.clone(),
                values: self.convert_enum_values(e),
            })
            .collect()
    }

    fn convert_enum_values(&self, enm: &dml::Enum) -> Vec<InternalEnumValue> {
        enm.values()
            .map(|enum_value| InternalEnumValue {
                name: enum_value.name.clone(),
                database_name: enum_value.database_name.clone(),
            })
            .collect()
    }

    fn convert_models(&self) -> Vec<ModelTemplate> {
        self.datamodel
            .models()
            .filter(|model| !model.is_ignored)
            .filter(|model| model.is_supported())
            .map(|model| ModelTemplate {
                name: model.name.clone(),
                is_embedded: model.is_embedded,
                fields: self.convert_fields(model),
                manifestation: model.database_name().map(|s| s.to_owned()),
                id_field_names: model.id_fields.clone(),
                indexes: self.convert_indexes(model),
                supports_create_operation: model.supports_create_operation(),
                dml_model: model.clone(),
            })
            .collect()
    }

    fn convert_fields(&self, model: &dml::Model) -> Vec<FieldTemplate> {
        model
            .fields()
            .filter(|field| !field.is_ignored())
            .filter_map(|field| match field {
                dml::Field::RelationField(rf) => {
                    let relation = self
                        .relations
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

                    Some(FieldTemplate::Relation(RelationFieldTemplate {
                        name: rf.name.clone(),
                        is_required: rf.is_required(),
                        is_list: rf.is_list(),
                        relation_name: relation.name.clone(),
                        relation_side: relation.relation_side(rf),
                        relation_info: rf.relation_info.clone(),
                        on_delete_default: rf.default_on_delete_action(),
                        on_update_default: rf.default_on_update_action(),
                    }))
                }
                dml::Field::ScalarField(sf) => {
                    if sf.type_identifier() == TypeIdentifier::Unsupported {
                        return None;
                    }

                    Some(FieldTemplate::Scalar(ScalarFieldTemplate {
                        name: sf.name.clone(),
                        type_identifier: sf.type_identifier(),
                        is_required: sf.is_required(),
                        is_list: sf.is_list(),
                        is_unique: sf.is_unique(model),
                        is_id: sf.is_id(model),
                        is_auto_generated_int_id: sf.is_auto_generated_int_id(),
                        is_autoincrement: sf.is_auto_increment(),
                        behaviour: sf.behaviour(),
                        internal_enum: sf.internal_enum(self.datamodel),
                        db_name: sf.database_name.clone(),
                        arity: sf.arity,
                        default_value: sf.default_value.clone(),
                        native_type: sf.native_type(),
                    }))
                }
            })
            .collect()
    }

    fn convert_relations(&self) -> Vec<RelationTemplate> {
        self.relations
            .iter()
            .filter(|r| r.model_a.is_relation_supported(&r.field_a) && r.model_b.is_relation_supported(&r.field_b))
            .map(|r| RelationTemplate {
                name: r.name(),
                manifestation: r.manifestation(),
                model_a_name: r.model_a.name.clone(),
                model_b_name: r.model_b.name.clone(),
            })
            .collect()
    }

    fn convert_indexes(&self, model: &dml::Model) -> Vec<IndexTemplate> {
        model
            .indices
            .iter()
            .filter(|i| i.fields.len() > 1 && model.is_compound_index_supported(i)) // @@unique for 1 field are transformed to is_unique instead
            .map(|i| IndexTemplate {
                name: i.name.clone(),
                fields: i.fields.clone(),
                typ: match i.tpe {
                    dml::IndexType::Unique => IndexType::Unique,
                    dml::IndexType::Normal => IndexType::Normal,
                },
            })
            .collect()
    }

    pub fn calculate_relations(datamodel: &dml::Datamodel) -> Vec<TempRelationHolder> {
        let mut result = Vec::new();
        for model in datamodel.models().filter(|model| !model.is_ignored) {
            for field in model.relation_fields().filter(|field| !field.is_ignored) {
                let dml::RelationInfo {
                    to, references, name, ..
                } = &field.relation_info;

                let related_model = datamodel
                    .find_model(to)
                    .unwrap_or_else(|| panic!("Related model {} not found", to));

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
                    // SELF RELATION CASE
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
                let inline_on_model_a = TempManifestationHolder::Inline {
                    in_table_of_model: model_a.name.clone(),
                    field: field_a.clone(),
                    referenced_fields: referenced_fields_a.clone(),
                };
                let inline_on_model_b = TempManifestationHolder::Inline {
                    in_table_of_model: model_b.name.clone(),
                    field: field_b.clone(),
                    referenced_fields: referenced_fields_b.clone(),
                };
                let inline_on_this_model = TempManifestationHolder::Inline {
                    in_table_of_model: model.name.clone(),
                    field: field.clone(),
                    referenced_fields: references.clone(),
                };
                let inline_on_related_model = TempManifestationHolder::Inline {
                    in_table_of_model: related_model.name.clone(),
                    field: related_field.clone(),
                    referenced_fields: related_field_info.references.clone(),
                };

                let manifestation = match (field_a.is_list(), field_b.is_list()) {
                    (true, true) => TempManifestationHolder::Table,
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

                result.push(TempRelationHolder {
                    name: name.clone(),
                    model_a,
                    model_b,
                    field_a,
                    field_b,
                    manifestation,
                })
            }
        }

        result.into_iter().unique_by(|rel| rel.name()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct TempRelationHolder {
    pub name: String,
    pub model_a: dml::Model,
    pub model_b: dml::Model,
    pub field_a: dml::RelationField,
    pub field_b: dml::RelationField,
    pub manifestation: TempManifestationHolder,
}

#[derive(PartialEq, Debug, Clone)]
pub enum TempManifestationHolder {
    Inline {
        in_table_of_model: String,
        /// The relation field.
        field: dml::RelationField,
        /// The name of the (dml) fields referenced by the relation.
        referenced_fields: Vec<String>,
    },
    Table,
}

#[allow(unused)]
impl TempRelationHolder {
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
            TempManifestationHolder::Table => RelationLinkManifestation::RelationTable(RelationTable {
                table: self.table_name(),
                model_a_column: self.model_a_column(),
                model_b_column: self.model_b_column(),
            }),
            TempManifestationHolder::Inline { in_table_of_model, .. } => {
                RelationLinkManifestation::Inline(InlineRelation {
                    in_table_of_model_name: in_table_of_model.to_string(),
                })
            }
        }
    }
}

trait ModelConverterUtilities {
    // A model is supported if it has at least one indexed/unique field or compound index that's supported.
    fn is_supported(&self) -> bool;
    // Checks if a model has an indexed/unique field that's supported
    fn has_supported_indexed_field(&self) -> bool;
    // Checks if a model has a compound index that's supported
    fn has_supported_compound_index(&self) -> bool;
    // Checks if a relation is supported.
    // A relation is supported if none of its fk field are of type Unsupported
    fn is_relation_supported(&self, rf: &dml::RelationField) -> bool;
    // Checks if a compound index is supported
    // A compound index is supported is none of its member are of type Unsupported
    fn is_compound_index_supported(&self, index: &dml::IndexDefinition) -> bool;
    // Checks if a model can support the create operation.
    // It can't if it has a field of type `Unsupported` required and without a default value
    fn supports_create_operation(&self) -> bool;
}

impl ModelConverterUtilities for dml::Model {
    fn is_supported(&self) -> bool {
        self.has_supported_indexed_field() || self.has_supported_compound_index()
    }

    fn is_relation_supported(&self, rf: &dml::RelationField) -> bool {
        if rf.is_ignored {
            return false;
        }

        rf.relation_info.fields.iter().all(|fk_name| {
            let field = self.find_field(fk_name).unwrap();
            let is_supported = match field {
                dml::Field::ScalarField(sf) => sf.type_identifier() != TypeIdentifier::Unsupported,
                dml::Field::RelationField(_) => true,
            };

            is_supported && !field.is_ignored()
        })
    }

    fn supports_create_operation(&self) -> bool {
        let has_unsupported_field = self.fields.iter().any(|field| match field {
            dml::Field::ScalarField(sf) => {
                (sf.type_identifier() == TypeIdentifier::Unsupported || field.is_ignored())
                    && sf.is_required()
                    && sf.default_value.is_none()
            }
            _ => false,
        });

        !has_unsupported_field
    }

    fn has_supported_indexed_field(&self) -> bool {
        self.fields.iter().any(|field| {
            let is_supported_field = match field {
                dml::Field::ScalarField(sf) => sf.type_identifier() != TypeIdentifier::Unsupported,
                _ => false,
            };

            self.field_is_indexed(field.name()) && !field.is_ignored() && is_supported_field
        })
    }

    fn is_compound_index_supported(&self, index: &dml::IndexDefinition) -> bool {
        index.fields.iter().all(|field_name| {
            let field = self.find_field(field_name).unwrap();
            let is_supported = match field {
                dml::Field::ScalarField(sf) => sf.type_identifier() != TypeIdentifier::Unsupported,
                dml::Field::RelationField(_) => true,
            };

            is_supported && !field.is_ignored()
        })
    }

    fn has_supported_compound_index(&self) -> bool {
        self.indices.iter().any(|index| self.is_compound_index_supported(index))
    }
}

trait DatamodelFieldExtensions {
    fn type_identifier(&self) -> TypeIdentifier;
    fn is_unique(&self, model: &dml::Model) -> bool;
    fn is_id(&self, model: &dml::Model) -> bool;
    fn is_auto_generated_int_id(&self) -> bool;
    fn behaviour(&self) -> Option<FieldBehaviour>;
    fn internal_enum(&self, datamodel: &dml::Datamodel) -> Option<InternalEnum>;
    fn internal_enum_value(&self, enum_value: &dml::EnumValue) -> InternalEnumValue;
    fn native_type(&self) -> Option<NativeTypeInstance>;
}

impl DatamodelFieldExtensions for dml::ScalarField {
    fn type_identifier(&self) -> TypeIdentifier {
        match &self.field_type {
            dml::FieldType::Enum(x) => TypeIdentifier::Enum(x.clone()),
            dml::FieldType::Relation(_) => TypeIdentifier::String, // Todo: Unused
            dml::FieldType::Scalar(scalar, _, _) => (*scalar).into(),
            dml::FieldType::Unsupported(_) => TypeIdentifier::Unsupported,
        }
    }

    fn is_unique(&self, model: &dml::Model) -> bool {
        // transform @@unique for 1 field to is_unique
        let is_declared_as_unique_through_multi_field_unique = model
            .indices
            .iter()
            .any(|ixd| ixd.is_unique() && ixd.fields == vec![self.name.clone()]);

        self.is_unique || is_declared_as_unique_through_multi_field_unique
    }

    fn is_id(&self, model: &dml::Model) -> bool {
        // transform @@id for 1 field to is_id
        self.is_id || model.id_fields == vec![self.name.clone()]
    }

    fn is_auto_generated_int_id(&self) -> bool {
        let is_autogenerated_id = matches!(self.default_value, Some(DefaultValue::Expression(_)) if self.is_id);

        let is_an_int = self.type_identifier() == TypeIdentifier::Int;

        is_autogenerated_id && is_an_int
    }

    fn behaviour(&self) -> Option<FieldBehaviour> {
        if self.is_updated_at {
            Some(FieldBehaviour::UpdatedAt)
        } else {
            None
        }
    }

    fn internal_enum(&self, datamodel: &dml::Datamodel) -> Option<InternalEnum> {
        match self.field_type {
            dml::FieldType::Enum(ref name) => {
                datamodel
                    .enums()
                    .find(|e| e.name == name.clone())
                    .map(|e| InternalEnum {
                        name: e.name.clone(),
                        values: e.values().map(|v| self.internal_enum_value(v)).collect(),
                    })
            }
            _ => None,
        }
    }

    fn internal_enum_value(&self, enum_value: &dml::EnumValue) -> InternalEnumValue {
        InternalEnumValue {
            name: enum_value.name.clone(),
            database_name: enum_value.database_name.clone(),
        }
    }

    fn native_type(&self) -> Option<NativeTypeInstance> {
        match &self.field_type {
            datamodel::FieldType::Scalar(_, _, nt) => nt.clone(),
            _ => None,
        }
    }
}
