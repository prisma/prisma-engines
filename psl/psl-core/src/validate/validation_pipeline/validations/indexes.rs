use super::{constraint_namespace::ConstraintName, database_name::validate_db_name};
use crate::{
    datamodel_connector::{walker_ext_traits::*, ConnectorCapability},
    diagnostics::DatamodelError,
    validate::validation_pipeline::context::Context,
    PreviewFeature,
};
use itertools::Itertools;
use parser_database::{walkers::IndexWalker, IndexAlgorithm};

/// Different databases validate index and unique constraint names in a certain namespace.
/// Validates index and unique constraint names against the database requirements.
pub(super) fn has_a_unique_constraint_name(index: IndexWalker<'_>, names: &super::Names<'_>, ctx: &mut Context<'_>) {
    let name = index.constraint_name(ctx.connector);
    let model = index.model();

    for violation in names.constraint_namespace.constraint_name_scope_violations(
        model.model_id(),
        ConstraintName::Index(name.as_ref()),
        ctx,
    ) {
        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name()),
        );

        let from_arg = index
            .ast_attribute()
            .span_for_argument("map")
            .or_else(|| index.ast_attribute().span_for_argument("name"));

        let span = from_arg.unwrap_or(index.ast_attribute().span);

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            index.attribute_name(),
            span,
        ));
    }
}

/// The custom name argument makes its way into the generated client API. Therefore the name argument
/// needs to be unique per model. It can be found on the primary key or unique indexes.
pub(super) fn unique_index_has_a_unique_custom_name_per_model(
    index: IndexWalker<'_>,
    names: &super::Names<'_>,
    ctx: &mut Context<'_>,
) {
    let model = index.model();

    if let Some(name) = index.name() {
        if names
            .constraint_namespace
            .local_custom_name_scope_violations(model.model_id(), name.as_ref())
        {
            let message = format!(
                "The given custom name `{name}` has to be unique on the model. Please provide a different name for the `name` argument."
            );

            let from_arg = index.ast_attribute().span_for_argument("name");
            let span = from_arg.unwrap_or(index.ast_attribute().span);

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message,
                index.attribute_name(),
                span,
            ));
        }
    }
}

/// The database must support the index length prefix for it to be allowed in the data model.
pub(crate) fn field_length_prefix_supported(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if ctx
        .connector
        .has_capability(ConnectorCapability::IndexColumnLengthPrefixing)
    {
        return;
    }

    if index.scalar_field_attributes().any(|f| f.length().is_some()) {
        let message = "The length argument is not supported in an index definition with the current connector";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

/// `@@fulltext` attribute is not available without `fullTextIndex` preview feature.
pub(crate) fn fulltext_index_preview_feature_enabled(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if index.is_fulltext() {
        let message = "You must enable `fullTextIndex` preview feature to be able to define a @@fulltext index.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

/// `@@fulltext` should only be available if we support it in the database.
pub(crate) fn fulltext_index_supported(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if index.is_fulltext() {
        let message = "Defining fulltext indexes is not supported with the current connector.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

/// `@@fulltext` index columns should not define `length` argument.
pub(crate) fn fulltext_columns_should_not_define_length(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if !index.is_fulltext() {
        return;
    }

    if index.scalar_field_attributes().any(|f| f.length().is_some()) {
        let message = "The length argument is not supported in a @@fulltext attribute.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

/// Only MongoDB supports sort order in a fulltext index.
pub(crate) fn fulltext_column_sort_is_supported(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if !index.is_fulltext() {
        return;
    }

    if ctx
        .connector
        .has_capability(ConnectorCapability::SortOrderInFullTextIndex)
    {
        return;
    }

    if index.scalar_field_attributes().any(|f| f.sort_order().is_some()) {
        let message = "The sort argument is not supported in a @@fulltext attribute in the current connector.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

/// Mongo wants all text keys to be bundled together, so e.g. this doesn't work:
///
/// ```ignore
/// @@fulltext([a(sort: Asc), b, c(sort: Asc), d])
/// ```
pub(crate) fn fulltext_text_columns_should_be_bundled_together(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if !ctx.connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if !index.is_fulltext() {
        return;
    }

    if !ctx
        .connector
        .has_capability(ConnectorCapability::SortOrderInFullTextIndex)
    {
        return;
    }

    enum State {
        // The empty state in the beginning. Must move to another state in every case.
        Init,
        // We've only had sorted fields so far.
        SortParamHead,
        // The bundle of text fields, we can have only one per index.
        TextFieldBundle,
        // The sort params after one text bundle.
        SortParamTail,
    }

    let mut state = State::Init;

    for field in index.scalar_field_attributes() {
        state = match state {
            State::Init if field.sort_order().is_some() => State::SortParamHead,
            State::SortParamHead if field.sort_order().is_some() => State::SortParamHead,
            State::TextFieldBundle if field.sort_order().is_some() => State::SortParamTail,
            State::SortParamTail if field.sort_order().is_some() => State::SortParamTail,
            State::Init | State::SortParamHead | State::TextFieldBundle => State::TextFieldBundle,
            State::SortParamTail => {
                let message = "All index fields must be listed adjacently in the fields argument.";

                ctx.push_error(DatamodelError::new_attribute_validation_error(
                    message,
                    index.attribute_name(),
                    index.ast_attribute().span,
                ));

                return;
            }
        }
    }
}

/// The ordering is only possible with `BTree` access method.
pub(crate) fn hash_index_must_not_use_sort_param(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.connector.supports_index_type(IndexAlgorithm::Hash) {
        return;
    }

    if !index.algorithm().map(|alg| alg.is_hash()).unwrap_or(false) {
        return;
    }

    if index.scalar_field_attributes().any(|f| f.sort_order().is_some()) {
        let message = "Hash type does not support sort option.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

pub(super) fn has_valid_mapped_name(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    validate_db_name(
        index.model().name(),
        index.ast_attribute(),
        index.mapped_name(),
        ctx,
        !index.is_defined_on_field(),
    )
}

pub(super) fn has_fields(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if index.fields().len() > 0 {
        return;
    }

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        "The list of fields in an index cannot be empty. Please specify at least one field.",
        index.attribute_name(),
        index.ast_attribute().span,
    ))
}

pub(crate) fn supports_clustering_setting(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.connector.has_capability(ConnectorCapability::ClusteringSetting) {
        return;
    }

    if index.clustered().is_none() {
        return;
    }

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        "Defining clustering is not supported in the current connector.",
        index.attribute_name(),
        index.ast_attribute().span,
    ))
}

pub(crate) fn clustering_can_be_defined_only_once(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !ctx.connector.has_capability(ConnectorCapability::ClusteringSetting) {
        return;
    }

    if index.clustered() != Some(true) {
        return;
    }

    if let Some(pk) = index.model().primary_key() {
        if matches!(pk.clustered(), Some(true) | None) {
            ctx.push_error(DatamodelError::new_attribute_validation_error(
                "A model can only hold one clustered index or key.",
                index.attribute_name(),
                index.ast_attribute().span,
            ));
        }
    }

    for other in index.model().indexes() {
        if other.attribute_id() == index.attribute_id() {
            continue;
        }

        if other.clustered() != Some(true) {
            continue;
        }

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "A model can only hold one clustered index.",
            index.attribute_name(),
            index.ast_attribute().span,
        ));

        return;
    }
}

/// Is the index algorithm supported by the current connector.
pub(crate) fn index_algorithm_is_supported(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    let algo = match index.algorithm() {
        Some(algo) => algo,
        None => return,
    };

    if ctx.connector.supports_index_type(algo) {
        return;
    }

    let message = "The given index type is not supported with the current connector";

    let span = index
        .ast_attribute()
        .span_for_argument("type")
        .unwrap_or_else(|| index.ast_attribute().span);

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        message,
        index.attribute_name(),
        span,
    ));
}

/// You can use `ops` argument only with a normal index.
pub(crate) fn opclasses_are_not_allowed_with_other_than_normal_indices(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if index.is_normal() {
        return;
    }

    for field in index.scalar_field_attributes() {
        if field.operator_class().is_none() {
            continue;
        }

        let message = "Operator classes can only be defined to fields in an @@index attribute.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));

        return;
    }
}

pub(crate) fn composite_type_in_compound_unique_index(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !index.is_unique() {
        return;
    }

    let composite_type = index
        .fields()
        .find(|f| f.scalar_field_type().as_composite_type().is_some());

    if index.fields().len() > 1 && composite_type.is_some() {
        let message = format!(
            "Prisma does not currently support composite types in compound unique indices, please remove {:?} from the index. See https://pris.ly/d/mongodb-composite-compound-indices for more details",
            composite_type.unwrap().name()
        );
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            index.attribute_name(),
            index.ast_attribute().span,
        ));
    }
}

pub(super) fn unique_client_name_does_not_clash_with_field(index: IndexWalker<'_>, ctx: &mut Context<'_>) {
    if !index.is_unique() {
        return;
    }

    // Only compound indexes clash.
    if index.fields().len() <= 1 {
        return;
    }

    let idx_client_name = index.all_field_names().join("_");

    if index.model().scalar_fields().any(|f| f.name() == idx_client_name) {
        let attr_name = index.attribute_name();

        let container_type = if index.model().ast_model().is_view() {
            "view"
        } else {
            "model"
        };

        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!("The field `{idx_client_name}` clashes with the `{attr_name}` name. Please resolve the conflict by providing a custom id name: `{attr_name}([...], name: \"custom_name\")`"),
            container_type,
            index.model().name(),
            index.ast_attribute().span,
        ));
    }
}
