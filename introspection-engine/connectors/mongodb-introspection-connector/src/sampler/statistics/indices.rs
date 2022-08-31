use super::Name;
use crate::sampler::field_type::FieldType;
use convert_case::{Case, Casing};
use introspection_connector::Warning;
use mongodb_schema_describer::{IndexFieldProperty, IndexWalker};
use psl::dml::{self, WithDatabaseName, WithName};
use std::collections::BTreeMap;

/// Add described indices to the models.
pub(super) fn add_to_models(
    models: &mut BTreeMap<String, dml::Model>,
    types: &mut BTreeMap<String, dml::CompositeType>,
    indices: &mut BTreeMap<String, Vec<IndexWalker<'_>>>,
    warnings: &mut Vec<Warning>,
) {
    let mut fields_with_unknown_type = Vec::new();

    for (model_name, model) in models.iter_mut() {
        for index in indices.remove(model_name).into_iter().flat_map(|i| i.into_iter()) {
            let defined_on_field = index.fields().len() == 1 && !index.fields().any(|f| f.name().contains('.'));

            add_missing_fields_from_index(model, index, &mut fields_with_unknown_type);
            add_missing_types_from_index(types, model, index, &mut fields_with_unknown_type);

            let fields = index
                .fields()
                .map(|f| {
                    let mut path = Vec::new();
                    let mut splitted_name = f.name().split('.');
                    let mut next_type: Option<&dml::CompositeType> = None;

                    if let Some(field_name) = splitted_name.next() {
                        next_type = model
                            .fields()
                            .find(|f| f.database_name() == Some(field_name) || f.name() == field_name)
                            .and_then(|f| f.as_scalar_field())
                            .and_then(|f| match &f.field_type {
                                dml::FieldType::CompositeType(ref r#type) => types.get(r#type),
                                _ => None,
                            });

                        let name = super::sanitize_string(field_name).unwrap_or_else(|| field_name.to_owned());

                        path.push((name, None));
                    }

                    for field_name in splitted_name {
                        let ct_name = next_type.as_ref().map(|ct| ct.name.clone());
                        let name = super::sanitize_string(field_name).unwrap_or_else(|| field_name.to_owned());
                        path.push((name, ct_name));

                        next_type = next_type.as_ref().and_then(|ct| {
                            ct.fields
                                .iter()
                                .find(|f| f.database_name.as_deref() == Some(field_name) || f.name == field_name)
                                .and_then(|f| match &f.r#type {
                                    dml::CompositeTypeFieldType::CompositeType(ref r#type) => types.get(r#type),
                                    _ => None,
                                })
                        });
                    }

                    dml::IndexField {
                        path,
                        sort_order: match f.property {
                            IndexFieldProperty::Text => None,
                            IndexFieldProperty::Ascending => Some(dml::SortOrder::Asc),
                            IndexFieldProperty::Descending => Some(dml::SortOrder::Desc),
                        },
                        length: None,
                        operator_class: None,
                    }
                })
                .collect();

            let tpe = match index.r#type() {
                mongodb_schema_describer::IndexType::Normal => dml::IndexType::Normal,
                mongodb_schema_describer::IndexType::Unique => dml::IndexType::Unique,
                mongodb_schema_describer::IndexType::Fulltext => dml::IndexType::Fulltext,
            };

            model.add_index(dml::IndexDefinition {
                fields,
                tpe,
                defined_on_field,
                db_name: Some(index.name().to_string()),
                name: None,
                algorithm: None,
                clustered: None,
            });
        }
    }

    if !fields_with_unknown_type.is_empty() {
        warnings.push(crate::warnings::fields_with_unknown_types(&fields_with_unknown_type));
    }
}

/// If an index points to a field not in the model, we'll add it as an unknown
/// field with type `Json?`.
fn add_missing_fields_from_index(
    model: &mut dml::Model,
    index: IndexWalker<'_>,
    unknown_fields: &mut Vec<(Name, String)>,
) {
    let missing_fields_in_models: Vec<_> = index
        .fields()
        .filter(|indf| !indf.name().contains('.'))
        .filter(|indf| {
            !model
                .fields
                .iter()
                .any(|mf| mf.name() == indf.name() || mf.database_name() == Some(indf.name()))
        })
        .cloned()
        .collect();

    for field in missing_fields_in_models {
        let docs = String::from("Field referred in an index, but found no data to define the type.");
        unknown_fields.push((Name::Model(model.name().clone()), field.name.clone()));

        let (name, database_name) = match super::sanitize_string(field.name()) {
            Some(name) => (name, Some(field.name)),
            None => (field.name, None),
        };

        let sf = dml::ScalarField {
            name,
            field_type: FieldType::Json.into(),
            arity: dml::FieldArity::Optional,
            database_name,
            default_value: None,
            documentation: Some(docs),
            is_generated: false,
            is_updated_at: false,
            is_commented_out: false,
            is_ignored: false,
        };

        model.fields.push(dml::Field::ScalarField(sf));
    }
}

/// If an index points to a composite field, and we have no data to describe it,
/// we create a stub type and a field of type `Json?` to mark the unknown field.
fn add_missing_types_from_index(
    types: &mut BTreeMap<String, dml::CompositeType>,
    model: &mut dml::Model,
    index: IndexWalker<'_>,
    unknown_fields: &mut Vec<(Name, String)>,
) {
    let composite_fields = index.fields().filter(|indf| indf.name().contains('.'));

    for indf in composite_fields {
        let path_length = indf.name().split('.').count();
        let mut splitted_name = indf.name().split('.').enumerate();
        let mut next_type = None;

        if let Some((_, field_name)) = splitted_name.next() {
            let sf = model
                .fields()
                .find(|f| f.database_name() == Some(field_name) || f.name() == field_name)
                .and_then(|f| f.as_scalar_field());

            next_type = match sf {
                Some(sf) => sf.field_type.as_composite_type().map(|tyname| tyname.to_owned()),
                None => {
                    let docs = String::from("Field referred in an index, but found no data to define the type.");

                    let (field_name, database_name) = match super::sanitize_string(field_name) {
                        Some(name) => (name, Some(field_name.to_string())),
                        None => (field_name.to_string(), None),
                    };

                    let type_name = format!("{}_{}", model.name, field_name).to_case(Case::Pascal);

                    let ct = dml::CompositeType {
                        name: type_name.clone(),
                        fields: Vec::new(),
                    };

                    types.insert(type_name.clone(), ct);

                    let sf = dml::ScalarField {
                        name: field_name,
                        field_type: dml::FieldType::CompositeType(type_name.clone()),
                        arity: dml::FieldArity::Optional,
                        database_name,
                        default_value: None,
                        documentation: Some(docs),
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    };

                    model.fields.push(dml::Field::ScalarField(sf));

                    Some(type_name)
                }
            };
        }

        for (i, field_name) in splitted_name {
            let type_name = match next_type.take() {
                Some(name) => name,
                None => continue,
            };
            let ct = types.get(&type_name).unwrap();

            let cf = ct
                .fields
                .iter()
                .find(|f| f.database_name.as_deref() == Some(field_name) || f.name == field_name);

            next_type = match (cf, cf.and_then(|cf| cf.r#type.as_composite_type())) {
                (Some(_), _) if i + 1 == path_length => None,
                (Some(_), Some(type_name)) => Some(type_name.to_string()),
                (None, _) | (_, None) => {
                    let docs = String::from("Field referred in an index, but found no data to define the type.");

                    let (field_name, database_name) = match super::sanitize_string(field_name) {
                        Some(name) => (name, Some(field_name.to_string())),
                        None => (field_name.to_string(), None),
                    };

                    let (r#type, new_type_name) = if i + 1 < path_length {
                        let type_name = format!("{}_{}", type_name, field_name).to_case(Case::Pascal);

                        types.insert(
                            type_name.clone(),
                            dml::CompositeType {
                                name: type_name.clone(),
                                fields: Vec::new(),
                            },
                        );

                        (
                            dml::CompositeTypeFieldType::CompositeType(type_name.clone()),
                            Some(type_name.clone()),
                        )
                    } else {
                        unknown_fields.push((Name::CompositeType(type_name.clone()), field_name.clone()));

                        (dml::CompositeTypeFieldType::Scalar(dml::ScalarType::Json, None), None)
                    };

                    let ct = types.get_mut(&type_name).unwrap();

                    ct.fields.push(dml::CompositeTypeField {
                        name: field_name,
                        r#type,
                        arity: dml::FieldArity::Optional,
                        database_name,
                        documentation: Some(docs),
                        default_value: None,
                        is_commented_out: false,
                    });

                    new_type_name
                }
            }
        }
    }
}
