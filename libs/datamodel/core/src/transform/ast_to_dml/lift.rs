use crate::{
    ast::{self, WithName},
    diagnostics::{DatamodelError, Diagnostics},
    dml,
    transform::{
        ast_to_dml::db::{ParserDatabase, ScalarFieldType},
        helpers::ValueValidator,
    },
    Datasource,
};
use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Helper for lifting a datamodel.
///
/// When lifting, the AST is converted to the real datamodel, and additional
/// semantics are attached.
pub struct LiftAstToDml<'a> {
    db: &'a ParserDatabase<'a>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> LiftAstToDml<'a> {
    /// Creates a new instance, with all builtin attributes and
    /// the attributes defined by the given sources registered.
    ///
    /// The attributes defined by the given sources will be namespaced.
    pub(crate) fn new(db: &'a ParserDatabase<'a>, diagnostics: &'a mut Diagnostics) -> LiftAstToDml<'a> {
        LiftAstToDml { db, diagnostics }
    }

    pub fn lift(&mut self) -> dml::Datamodel {
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
    fn lift_model(&mut self, model_id: ast::ModelId, ast_model: &ast::Model) -> dml::Model {
        let mut model = dml::Model::new(ast_model.name.name.clone(), None);
        let model_data = self.db.get_model_data(&model_id).unwrap();

        model.documentation = ast_model.documentation.clone().map(|comment| comment.text);
        model.database_name = self.db.get_model_database_name(model_id).map(String::from);
        model.is_ignored = model_data.is_ignored;

        model.id_fields = model_data
            .id_fields
            .as_ref()
            .filter(|_| model_data.id_source_field.is_none())
            .map(|fields| {
                fields
                    .iter()
                    .map(|id| self.db.ast()[model_id][*id].name.name.clone())
                    .collect()
            })
            .unwrap_or_default();

        model.indices = model_data
            .indexes
            .iter()
            .filter(|idx| idx.source_field.is_none())
            .map(|idx| dml::IndexDefinition {
                name: idx.name.map(String::from),
                fields: idx
                    .fields
                    .iter()
                    .map(|id| self.db.ast()[model_id][*id].name.name.clone())
                    .collect(),
                tpe: match idx.is_unique {
                    true => dml::IndexType::Unique,
                    false => dml::IndexType::Normal,
                },
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
            let mut attributes = Vec::with_capacity(ast_field.attributes.len());

            let field_type = self.lift_scalar_field_type(ast_field, &scalar_field_data.r#type, &mut attributes);

            attributes.extend(ast_field.attributes.iter().cloned());

            let mut field = dml::ScalarField::new(&ast_field.name.name, arity, field_type);

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.is_ignored = scalar_field_data.is_ignored;
            field.is_id = model_data.id_source_field == Some(field_id);
            field.is_updated_at = scalar_field_data.is_updated_at;
            field.is_unique = model_data.indexes.iter().any(|idx| idx.source_field == Some(field_id));
            field.database_name = self.db.get_field_database_name(model_id, field_id).map(String::from);
            field.default_value = scalar_field_data.default.clone();

            field_ids_for_sorting.insert(&ast_field.name.name, field_id);
            model.add_field(dml::Field::ScalarField(field));
        }

        for (field_id, relation_field) in self.db.iter_model_relation_fields(model_id) {
            let ast_field = &ast_model[field_id];
            let arity = self.lift_field_arity(&ast_field.arity);
            let target_model = &self.db.ast()[relation_field.referenced_model];
            let relation_info = dml::RelationInfo::new(target_model.name());

            let mut field = dml::RelationField::new(&ast_field.name.name, arity, arity, relation_info);

            field.supports_restrict_action(
                active_connector.supports_referential_action(dml::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(active_connector.emulates_referential_actions());

            field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
            field.relation_info.name = relation_field.name.map(String::from).unwrap_or_default();
            field.relation_info.on_delete = relation_field.on_delete;
            field.relation_info.on_update = relation_field.on_update;
            field.is_ignored = relation_field.is_ignored;

            field.relation_info.references = relation_field
                .references
                .as_ref()
                .map(|references| references.iter().map(|s| target_model[*s].name().to_owned()).collect())
                .unwrap_or_default();

            field.relation_info.fields = relation_field
                .fields
                .as_ref()
                .map(|fields| {
                    fields
                        .iter()
                        .map(|id| self.db.ast()[model_id][*id].name.name.clone())
                        .collect()
                })
                .unwrap_or_default();

            field_ids_for_sorting.insert(&ast_field.name.name, field_id);
            model.add_field(dml::Field::RelationField(field))
        }

        model.fields.sort_by_key(|f| field_ids_for_sorting.get(f.name()));
        model
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&mut self, enum_id: ast::EnumId, ast_enum: &ast::Enum) -> dml::Enum {
        let mut en = dml::Enum::new(&ast_enum.name.name, vec![]);

        if !self.db.active_connector().supports_enums() {
            self.diagnostics.push_error(DatamodelError::new_validation_error(
                &format!(
                    "You defined the enum `{}`. But the current connector does not support enums.",
                    &ast_enum.name.name
                ),
                ast_enum.span,
            ));
            return en;
        }

        for (value_idx, ast_enum_value) in ast_enum.values.iter().enumerate() {
            en.add_value(self.lift_enum_value(ast_enum_value, enum_id, value_idx as u32));
        }

        if en.values.is_empty() {
            self.diagnostics.push_error(DatamodelError::new_validation_error(
                "An enum must have at least one value.",
                ast_enum.span,
            ))
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
        &mut self,
        ast_field: &ast::Field,
        scalar_field_type: &ScalarFieldType,
        collected_attributes: &mut Vec<ast::Attribute>,
    ) -> dml::FieldType {
        match scalar_field_type {
            ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].name.name;
                dml::FieldType::Enum(enum_name.to_owned())
            }
            ScalarFieldType::Unsupported => lift_unsupported_field_type(
                ast_field,
                ast_field.field_type.as_unsupported().unwrap().0,
                self.db.datasource(),
                self.diagnostics,
            ),
            ScalarFieldType::Alias(top_id) => {
                let alias = &self.db.ast()[*top_id];
                collected_attributes.extend(alias.attributes.iter().cloned());
                self.lift_scalar_field_type(alias, self.db.alias_scalar_field_type(top_id), collected_attributes)
            }
            ScalarFieldType::BuiltInScalar(scalar_type) => {
                let native_type = self
                    .db
                    .datasource()
                    .and_then(|datasource| lift_native_type(ast_field, scalar_type, datasource, self.diagnostics));
                dml::FieldType::Scalar(*scalar_type, None, native_type)
            }
        }
    }
}

fn lift_native_type(
    ast_field: &ast::Field,
    scalar_type: &dml::ScalarType,
    datasource: &Datasource,
    diagnostics: &mut Diagnostics,
) -> Option<dml::NativeTypeInstance> {
    let connector = &datasource.active_connector;
    let prefix = format!("{}{}", datasource.name, ".");

    let type_specifications_with_invalid_datasource_name = ast_field
        .attributes
        .iter()
        .filter(|dir| dir.name.name.contains('.') && !dir.name.name.starts_with(&prefix))
        .collect_vec();

    if !type_specifications_with_invalid_datasource_name.is_empty() {
        let incorrect_type_specification = type_specifications_with_invalid_datasource_name.first().unwrap();
        let mut type_specification_name_split = incorrect_type_specification.name.name.split('.');
        let given_prefix = type_specification_name_split.next().unwrap();
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::InvalidPrefixForNativeTypes {
                given_prefix: String::from(given_prefix),
                expected_prefix: datasource.name.clone(),
                suggestion: format!("{}{}", prefix, type_specification_name_split.next().unwrap()),
            })
            .to_string(),
            incorrect_type_specification.span,
        ));
        return None;
    }

    let type_specifications = ast_field
        .attributes
        .iter()
        .filter(|dir| dir.name.name.starts_with(&prefix))
        .collect_vec();

    let type_specification = type_specifications.first();

    if type_specifications.len() > 1 {
        diagnostics.push_error(DatamodelError::new_duplicate_attribute_error(
            &prefix,
            type_specification.unwrap().span,
        ));
        return None;
    }

    // convert arguments to string if possible
    let number_args = type_specification.map(|dir| dir.arguments.clone());
    let args = if let Some(number) = number_args {
        number
            .iter()
            .map(|arg| ValueValidator::new(&arg.value).raw())
            .collect_vec()
    } else {
        vec![]
    };

    let x = type_specification.map(|dir| dir.name.name.trim_start_matches(&prefix))?;
    let constructor = if let Some(cons) = connector.find_native_type_constructor(x) {
        cons
    } else {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: x.parse().unwrap(),
                connector_name: datasource.active_provider.clone(),
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    };

    let number_of_args = args.len();

    if number_of_args < constructor._number_of_args
        || ((number_of_args > constructor._number_of_args) && constructor._number_of_optional_args == 0)
    {
        diagnostics.push_error(DatamodelError::new_argument_count_missmatch_error(
            x,
            constructor._number_of_args,
            number_of_args,
            type_specification.unwrap().span,
        ));
        return None;
    }

    if number_of_args > constructor._number_of_args + constructor._number_of_optional_args
        && constructor._number_of_optional_args > 0
    {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::OptionalArgumentCountMismatchError {
                native_type: x.parse().unwrap(),
                optional_count: constructor._number_of_optional_args,
                given_count: number_of_args,
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    }

    // check for compatibility with scalar type
    if !constructor.prisma_types.contains(scalar_type) {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                native_type: x.parse().unwrap(),
                field_type: scalar_type.to_string(),
                expected_types: constructor.prisma_types.iter().map(|s| s.to_string()).join(" or "),
            })
            .to_string(),
            type_specification.unwrap().span,
        ));
        return None;
    }

    match connector.parse_native_type(x, args) {
        Err(connector_error) => {
            diagnostics.push_error(DatamodelError::new_connector_error(
                &connector_error.to_string(),
                type_specification.unwrap().span,
            ));
            None
        }
        Ok(parsed_native_type) => Some(parsed_native_type),
    }
}

fn lift_unsupported_field_type(
    ast_field: &ast::Field,
    unsupported_lit: &str,
    source: Option<&Datasource>,
    diagnostics: &mut Diagnostics,
) -> dml::FieldType {
    static TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?x)
    ^                           # beginning of the string
    (?P<prefix>[^(]+)           # a required prefix that is any character until the first opening brace
    (?:\((?P<params>.*?)\))?    # (optional) an opening parenthesis, a closing parenthesis and captured params in-between
    (?P<suffix>.+)?             # (optional) captured suffix after the params until the end of the string
    $                           # end of the string
    "#).unwrap()
    });

    if let Some(source) = source {
        let connector = &source.active_connector;

        if let Some(captures) = TYPE_REGEX.captures(unsupported_lit) {
            let prefix = captures.name("prefix").unwrap().as_str().trim();

            let params = captures.name("params");
            let args = match params {
                None => vec![],
                Some(params) => params.as_str().split(',').map(|s| s.trim().to_string()).collect(),
            };

            if let Ok(native_type) = connector.parse_native_type(prefix, args) {
                let prisma_type = connector.scalar_type_for_native_type(native_type.serialized_native_type.clone());

                let msg = format!(
                        "The type `Unsupported(\"{}\")` you specified in the type definition for the field `{}` is supported as a native type by Prisma. Please use the native type notation `{} @{}.{}` for full support.",
                        unsupported_lit, ast_field.name.name, prisma_type.to_string(), &source.name, native_type.render()
                    );

                diagnostics.push_error(DatamodelError::new_validation_error(&msg, ast_field.span));
            }
        }
    }

    dml::FieldType::Unsupported(unsupported_lit.into())
}
