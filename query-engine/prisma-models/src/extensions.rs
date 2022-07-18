use crate::{InternalEnum, InternalEnumValue, TypeIdentifier};
use datamodel::dml::{self, Ignorable, NativeTypeInstance};

pub trait ModelConverterUtilities {
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
                dml::Field::CompositeField(_) => false,
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
        index.fields.iter().all(|field| {
            let field = self.find_field(&field.path.first().unwrap().0).unwrap();
            let is_supported = match field {
                dml::Field::ScalarField(sf) => sf.type_identifier() != TypeIdentifier::Unsupported,
                dml::Field::RelationField(_) => true,
                dml::Field::CompositeField(_) => true,
            };

            is_supported && !field.is_ignored()
        })
    }

    fn has_supported_compound_index(&self) -> bool {
        self.indices.iter().any(|index| self.is_compound_index_supported(index))
    }
}

pub trait DatamodelFieldExtensions {
    fn type_identifier(&self) -> TypeIdentifier;
    fn internal_enum(&self, datamodel: &dml::Datamodel) -> Option<InternalEnum>;
    fn internal_enum_value(&self, enum_value: &dml::EnumValue) -> InternalEnumValue;
    fn native_type(&self) -> Option<NativeTypeInstance>;
}

impl DatamodelFieldExtensions for dml::ScalarField {
    fn type_identifier(&self) -> TypeIdentifier {
        match &self.field_type {
            dml::FieldType::CompositeType(_) => todo!("composite type support in datamodel_converter"),
            dml::FieldType::Enum(x) => TypeIdentifier::Enum(x.clone()),
            dml::FieldType::Relation(_) => TypeIdentifier::String, // Todo: Unused
            dml::FieldType::Scalar(scalar, _, _) => (*scalar).into(),
            dml::FieldType::Unsupported(_) => TypeIdentifier::Unsupported,
        }
    }

    fn internal_enum(&self, datamodel: &dml::Datamodel) -> Option<InternalEnum> {
        match self.field_type {
            dml::FieldType::Enum(ref name) => datamodel.enums().find(|e| e.name == *name).map(|e| InternalEnum {
                name: e.name.clone(),
                values: e.values().map(|v| self.internal_enum_value(v)).collect(),
            }),
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
            dml::FieldType::Scalar(_, _, nt) => nt.clone(),
            _ => None,
        }
    }
}

impl DatamodelFieldExtensions for dml::CompositeTypeField {
    fn type_identifier(&self) -> TypeIdentifier {
        match &self.r#type {
            dml::CompositeTypeFieldType::CompositeType(_) => {
                unreachable!("Composite fields should not use type identifiers")
            }
            dml::CompositeTypeFieldType::Scalar(scalar, _, _) => (*scalar).into(),
            dml::CompositeTypeFieldType::Enum(e) => TypeIdentifier::Enum(e.clone()),
            dml::CompositeTypeFieldType::Unsupported(_) => TypeIdentifier::Unsupported,
        }
    }

    fn internal_enum(&self, datamodel: &dml::Datamodel) -> Option<InternalEnum> {
        match self.r#type {
            dml::CompositeTypeFieldType::Enum(ref enum_name) => {
                datamodel.enums().find(|e| e.name == *enum_name).map(|e| InternalEnum {
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
        match &self.r#type {
            dml::CompositeTypeFieldType::Scalar(_, _, nt) => nt.clone(),
            _ => None,
        }
    }
}
