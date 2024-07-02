use log::*;
use lsp_types::{Location, ReferenceParams, Url};
use psl::{
    diagnostics::FileId,
    error_tolerant_parse_configuration,
    parser_database::ParserDatabase,
    schema_ast::ast::{
        AttributePosition, CompositeTypePosition, EnumPosition, Field, FieldId, FieldPosition, FieldType, Identifier,
        ModelId, ModelPosition, SchemaPosition, SourcePosition, Top, WithIdentifier, WithName,
    },
    Diagnostics, SourceFile,
};

use crate::{offsets::position_to_offset, span_to_range, LSPContext};

pub(super) type ReferencesContext<'a> = LSPContext<'a, ReferenceParams>;

pub(crate) fn empty_references() -> Vec<Location> {
    Vec::new()
}

fn empty_identifiers<'ast>() -> impl Iterator<Item = &'ast Identifier> {
    std::iter::empty()
}

pub(crate) fn references(schema_files: Vec<(String, SourceFile)>, params: ReferenceParams) -> Vec<Location> {
    let (_, config, _) = error_tolerant_parse_configuration(&schema_files);

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(&schema_files, &mut diag)
    };

    let Some(initiating_file_id) = db.file_id(params.text_document_position.text_document.uri.as_str()) else {
        warn!("Initating file name is not found in the schema");
        return empty_references();
    };

    let initiating_doc = db.source(initiating_file_id);

    let position = if let Some(pos) = position_to_offset(&params.text_document_position.position, initiating_doc) {
        pos
    } else {
        warn!("Received a position outside of the document boundaries in ReferenceParams");
        return empty_references();
    };

    let target_position = db.ast(initiating_file_id).find_at_position(position);

    let ctx = ReferencesContext {
        db: &db,
        config: &config,
        initiating_file_id,
        params: &params,
    };

    reference_locations_for_target(ctx, target_position)
}

fn reference_locations_for_target(ctx: ReferencesContext<'_>, target: SchemaPosition) -> Vec<Location> {
    let identifiers: Vec<&Identifier> = match target {
        // Blocks
        SchemaPosition::Model(model_id, ModelPosition::Name(name)) => {
            let model = ctx.db.walk((ctx.initiating_file_id, model_id));

            std::iter::once(model.ast_model().identifier())
                .chain(find_where_used_as_field_type(&ctx, name))
                .collect()
        }
        SchemaPosition::Enum(enum_id, EnumPosition::Name(name)) => {
            let enm = ctx.db.walk((ctx.initiating_file_id, enum_id));

            std::iter::once(enm.ast_enum().identifier())
                .chain(find_where_used_as_field_type(&ctx, name))
                .collect()
        }
        SchemaPosition::CompositeType(composite_id, CompositeTypePosition::Name(name)) => {
            let ct = ctx.db.walk((ctx.initiating_file_id, composite_id));

            std::iter::once(ct.ast_composite_type().identifier())
                .chain(find_where_used_as_field_type(&ctx, name))
                .collect()
        }
        SchemaPosition::DataSource(_, SourcePosition::Name(name)) => find_where_used_as_top_name(&ctx, name)
            .into_iter()
            .chain(find_where_used_for_native_type(&ctx, name))
            .collect(),

        // Fields
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Type(r#type)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Type(r#type))) => {
            find_where_used_as_top_name(&ctx, r#type)
                .into_iter()
                .chain(find_where_used_as_field_type(&ctx, r#type))
                .collect()
        }

        // Attributes
        SchemaPosition::Model(
            model_id,
            ModelPosition::Field(
                field_id,
                FieldPosition::Attribute(_, _, AttributePosition::ArgumentValue(arg_name, arg_value)),
            ),
        ) => match arg_name {
            Some("fields") => find_where_used_as_field_name(&ctx, arg_value.as_str(), model_id, ctx.initiating_file_id)
                .into_iter()
                .collect(),
            Some("references") => {
                let field = &ctx.db.ast(ctx.initiating_file_id)[model_id][field_id];
                let referenced_model = field.field_type.name();

                let Some(ref_model_id) = ctx.db.find_model(referenced_model) else {
                    warn!("Could not find model with name: {}", referenced_model);
                    return empty_references();
                };

                find_where_used_as_field_name(&ctx, arg_value.as_str(), ref_model_id.id.1, ref_model_id.id.0)
                    .into_iter()
                    .collect()
            }
            _ => vec![],
        },

        // ? This might make more sense to add as a definition rather than a reference
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Attribute(name, _, _)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Attribute(name, _, _))) => {
            match ctx.datasource().map(|ds| &ds.name) {
                Some(ds_name) if name.contains(ds_name) => {
                    find_where_used_as_top_name(&ctx, ds_name).into_iter().collect()
                }
                _ => vec![],
            }
        }

        SchemaPosition::Model(
            model_id,
            ModelPosition::ModelAttribute(_attr_name, _, AttributePosition::ArgumentValue(_, arg_val)),
        ) => find_where_used_as_field_name(&ctx, arg_val.as_str(), model_id, ctx.initiating_file_id)
            .into_iter()
            .collect(),

        _ => vec![],
    };

    identifiers
        .iter()
        .filter_map(|ident| ident_to_location(ident, &ctx))
        .collect()
}

fn find_where_used_as_field_name<'ast>(
    ctx: &'ast ReferencesContext<'_>,
    name: &str,
    model_id: ModelId,
    file_id: FileId,
) -> Option<&'ast Identifier> {
    let model = ctx.db.walk((file_id, model_id));

    match model.scalar_fields().find(|field| field.name() == name) {
        Some(field) => Some(field.ast_field().identifier()),
        None => None,
    }
}

fn find_where_used_for_native_type<'ast>(
    ctx: &ReferencesContext<'ast>,
    name: &'ast str,
) -> impl Iterator<Item = &'ast Identifier> {
    fn find_native_type_locations<'ast>(
        name: &'ast str,
        fields: impl Iterator<Item = (FieldId, &'ast Field)> + 'ast,
    ) -> Box<dyn Iterator<Item = &'ast Identifier> + 'ast> {
        Box::new(fields.filter_map(move |field| {
            field
                .1
                .attributes
                .iter()
                .find(|attr| extract_ds_from_native_type(attr.name()) == name)
                .map(|attr| attr.identifier())
        }))
    }

    ctx.db.walk_tops().flat_map(move |top| match top.ast_top() {
        Top::CompositeType(composite_type) => find_native_type_locations(name, composite_type.iter_fields()),
        Top::Model(model) => find_native_type_locations(name, model.iter_fields()),

        Top::Enum(_) | Top::Source(_) | Top::Generator(_) => Box::new(empty_identifiers()),
    })
}

fn find_where_used_as_field_type<'ast>(
    ctx: &'ast ReferencesContext<'_>,
    name: &'ast str,
) -> impl Iterator<Item = &'ast Identifier> {
    fn get_relevent_identifiers<'a>(
        fields: impl Iterator<Item = (FieldId, &'a Field)>,
        name: &str,
    ) -> Vec<&'a Identifier> {
        fields
            .filter_map(|(_id, field)| match &field.field_type {
                FieldType::Supported(id) if id.name == name => Some(id),
                _ => None,
            })
            .collect()
    }

    ctx.db.walk_tops().flat_map(|top| match top.ast_top() {
        Top::Model(model) => get_relevent_identifiers(model.iter_fields(), name),
        Top::CompositeType(composite_type) => get_relevent_identifiers(composite_type.iter_fields(), name),
        // * Cannot contain field types
        Top::Enum(_) | Top::Source(_) | Top::Generator(_) => vec![],
    })
}

fn find_where_used_as_top_name<'ast>(ctx: &'ast ReferencesContext<'_>, name: &'ast str) -> Option<&'ast Identifier> {
    ctx.db.find_top(name).map(|top| top.ast_top().identifier())
}

fn extract_ds_from_native_type(attr_name: &str) -> &str {
    attr_name.split('.').next().unwrap_or("")
}

fn ident_to_location<'ast>(id: &'ast Identifier, ctx: &'ast ReferencesContext<'_>) -> Option<Location> {
    let file_id = id.span.file_id;

    let source = ctx.db.source(file_id);
    let range = span_to_range(id.span, source);
    let file_name = ctx.db.file_name(file_id);

    let uri = if let Ok(uri) = Url::parse(file_name) {
        uri
    } else {
        return None;
    };

    Some(Location { uri, range })
}
