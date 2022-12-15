use super::database_name::validate_db_name;
use crate::{
    ast,
    datamodel_connector::{walker_ext_traits::*, ConnectorCapability},
    diagnostics::DatamodelError,
    parser_database::ast::{WithName, WithSpan},
    validate::validation_pipeline::context::Context,
    PreviewFeature,
};
use parser_database::walkers::{ModelWalker, PrimaryKeyWalker};
use std::{borrow::Cow, collections::HashMap};

/// A model must have either a primary key, or a unique criterion
/// with no optional, commented-out or unsupported fields.
pub(super) fn has_a_strict_unique_criteria(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if model.is_ignored() {
        return;
    }

    let strict_criteria = model
        .unique_criterias()
        .find(|c| c.is_strict_criteria() && !c.has_unsupported_fields());

    if strict_criteria.is_some() {
        return;
    }

    let mut loose_criterias = model
        .unique_criterias()
        .map(|c| {
            let field_names = c.fields().map(|c| c.name());
            format!("- {}", field_names.collect::<Vec<_>>().join(", "))
        })
        .peekable();

    let msg =
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.";

    let msg = if loose_criterias.peek().is_some() {
        let suffix = format!(
            "The following unique criterias were not considered as they contain fields that are not required:\n{}",
            loose_criterias.collect::<Vec<_>>().join("\n"),
        );

        Cow::from(format!("{} {}", msg, suffix))
    } else {
        Cow::from(msg)
    };

    ctx.push_error(DatamodelError::new_model_validation_error(
        msg.as_ref(),
        model.name(),
        model.ast_model().span(),
    ))
}

/// A primary key name can be unique in different namespaces, depending on a database. Validates
/// model's primary key against the database requirements.
pub(super) fn has_a_unique_primary_key_name(model: ModelWalker<'_>, names: &super::Names<'_>, ctx: &mut Context<'_>) {
    let (pk, name): (PrimaryKeyWalker<'_>, Cow<'_, str>) = match model
        .primary_key()
        .and_then(|pk| pk.constraint_name(ctx.connector).map(|name| (pk, name)))
    {
        Some((pk, name)) => (pk, name),
        None => return,
    };

    validate_db_name(
        model.name(),
        pk.ast_attribute(),
        Some(&name),
        ctx,
        !pk.is_defined_on_field(),
    );

    for violation in names.constraint_namespace.constraint_name_scope_violations(
        model.model_id(),
        super::constraint_namespace::ConstraintName::PrimaryKey(name.as_ref()),
        ctx,
    ) {
        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name())
        );

        let span = pk
            .ast_attribute()
            .span_for_argument("map")
            .unwrap_or_else(|| pk.ast_attribute().span);

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            pk.attribute_name(),
            span,
        ));
    }
}

/// The custom name argument makes its way into the generated client API. Therefore the name argument
/// needs to be unique per model. It can be found on the primary key or unique indexes.
pub(super) fn has_a_unique_custom_primary_key_name_per_model(
    model: ModelWalker<'_>,
    names: &super::Names<'_>,
    ctx: &mut Context<'_>,
) {
    let pk = match model.primary_key() {
        Some(pk) => pk,
        None => return,
    };

    if let Some(name) = pk.name() {
        if names
            .constraint_namespace
            .local_custom_name_scope_violations(model.model_id(), name.as_ref())
        {
            let message = format!(
                "The given custom name `{}` has to be unique on the model. Please provide a different name for the `name` argument.",
                name,
            );

            let span = pk
                .ast_attribute()
                .span_for_argument("name")
                .unwrap_or_else(|| pk.ast_attribute().span);

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message,
                pk.attribute_name(),
                span,
            ));
        }
    }
}

/// The database must support the primary key length prefix for it to be allowed in the data model.
pub(crate) fn primary_key_length_prefix_supported(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if ctx
        .connector
        .has_capability(ConnectorCapability::IndexColumnLengthPrefixing)
    {
        return;
    }

    if let Some(pk) = model.primary_key() {
        if pk.scalar_field_attributes().any(|f| f.length().is_some()) {
            let message = "The length argument is not supported in the primary key with the current connector";
            let span = pk.ast_attribute().span;

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                message,
                pk.attribute_name(),
                span,
            ));
        }
    }
}

/// Not every database is allowing sort definition in the primary key.
pub(crate) fn primary_key_sort_order_supported(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if ctx
        .connector
        .has_capability(ConnectorCapability::PrimaryKeySortOrderDefinition)
    {
        return;
    }

    if let Some(pk) = model.primary_key() {
        if pk.scalar_field_attributes().any(|f| f.sort_order().is_some()) {
            let message = "The sort argument is not supported in the primary key with the current connector";
            let span = pk.ast_attribute().span;

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                message,
                pk.attribute_name(),
                span,
            ));
        }
    }
}

pub(crate) fn only_one_fulltext_attribute_allowed(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if ctx
        .connector
        .has_capability(ConnectorCapability::MultipleFullTextAttributesPerModel)
    {
        return;
    }

    let spans = model
        .indexes()
        .filter(|i| i.is_fulltext())
        .map(|i| i.ast_attribute().span)
        .collect::<Vec<_>>();

    if spans.len() > 1 {
        for span in spans {
            let message = "The current connector only allows one fulltext attribute per model";

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                message,
                "@@fulltext",
                span,
            ));
        }
    }
}

/// Does the connector support named and compound primary keys at all?
pub(crate) fn primary_key_connector_specific(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    let primary_key = if let Some(pk) = model.primary_key() {
        pk
    } else {
        return;
    };

    if primary_key.mapped_name().is_some() && !ctx.connector.supports_named_primary_keys() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            model.name(),
            model.ast_model().span(),
        ));
    }

    if primary_key.fields().len() > 1 && !ctx.connector.supports_compound_ids() {
        return ctx.push_error(DatamodelError::new_model_validation_error(
            "The current connector does not support compound ids.",
            model.name(),
            primary_key.ast_attribute().span,
        ));
    }
}

pub(super) fn connector_specific(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    ctx.connector.validate_model(model, ctx.relation_mode, ctx.diagnostics)
}

pub(super) fn id_has_fields(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    let id = if let Some(id) = model.primary_key() { id } else { return };

    if id.fields().len() > 0 {
        return;
    }

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        "The list of fields in an `@@id()` attribute cannot be empty. Please specify at least one field.",
        id.attribute_name(),
        id.ast_attribute().span,
    ))
}

pub(super) fn id_client_name_does_not_clash_with_field(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    let id = if let Some(id) = model.primary_key() { id } else { return };

    // Only compound ids clash.
    if id.fields().len() <= 1 {
        return;
    }

    let id_client_name = id.fields().map(|f| f.name()).collect::<Vec<_>>().join("_");
    if model.scalar_fields().any(|f| f.name() == id_client_name) {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!("The field `{id_client_name}` clashes with the `@@id` attribute's name. Please resolve the conflict by providing a custom id name: `@@id([...], name: \"custom_name\")`"),
            model.name(),
            id.ast_attribute().span,
        ));
    }
}

pub(super) fn schema_is_defined_in_the_datasource(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let datasource = match ctx.datasource {
        Some(ds) => ds,
        None => return,
    };

    let (schema_name, span) = match model.schema() {
        Some(tuple) => tuple,
        None => return,
    };

    if datasource.has_schema(schema_name) {
        return;
    }

    ctx.push_error(DatamodelError::new_static(
        "This schema is not defined in the datasource. Read more on `@@schema` at https://pris.ly/d/multi-schema",
        span,
    ))
}

pub(super) fn schema_attribute_supported_in_connector(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let (_, span) = match model.schema() {
        Some(tuple) => tuple,
        None => return,
    };

    ctx.push_error(DatamodelError::new_static(
        "@@schema is not supported on the current datasource provider",
        span,
    ));
}

pub(super) fn schema_attribute_missing(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::MultiSchema) {
        return;
    }

    let datasource = match ctx.datasource {
        Some(datasource) => datasource,
        None => return,
    };

    if datasource.schemas_span.is_none() {
        return;
    }

    if ctx.connector.is_provider("mysql") {
        return;
    }

    if model.schema().is_some() {
        return;
    }

    ctx.push_error(DatamodelError::new_static(
        "This model is missing an `@@schema` attribute.",
        model.ast_model().span(),
    ))
}

pub(super) fn database_name_clashes(ctx: &mut Context<'_>) {
    // (schema_name, model_database_name) -> ModelId
    let mut database_names: HashMap<(Option<&str>, &str), ast::ModelId> = HashMap::with_capacity(ctx.db.models_count());

    for model in ctx.db.walk_models() {
        let key = (model.schema().map(|(name, _)| name), model.database_name());
        match database_names.insert(key, model.model_id()) {
            // Two branches because we want to put the error on the @@map attribute, and it can be
            // on either model.
            Some(existing) if model.mapped_name().is_some() => {
                let existing_model_name = &ctx.db.ast()[existing].name();
                let attribute = model
                    .ast_model()
                    .attributes
                    .iter()
                    .find(|attr| attr.name() == "map")
                    .unwrap();

                ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
                    model.database_name(),
                    existing_model_name,
                    attribute.span(),
                ));
            }
            Some(existing) => {
                let existing_model = &ctx.db.ast()[existing];
                let attribute = existing_model
                    .attributes
                    .iter()
                    .find(|attr| attr.name() == "map")
                    .unwrap();

                ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
                    model.database_name(),
                    model.name(),
                    attribute.span(),
                ));
            }
            None => (),
        }
    }
}

pub(super) fn multischema_feature_flag_needed(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        return;
    }

    if let Some((_, span)) = model.schema() {
        ctx.push_error(DatamodelError::new_static(
            "@@schema is only available with the `multiSchema` preview feature.",
            span,
        ));
    }
}
