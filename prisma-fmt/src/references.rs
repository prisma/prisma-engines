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

pub(crate) fn references(schema_files: Vec<(String, SourceFile)>, params: ReferenceParams) -> Vec<Location> {
    info!("Finding references");

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
    info!("{:?}", target);

    match target {
        // Blocks
        SchemaPosition::Model(_, ModelPosition::Name(name))
        | SchemaPosition::Enum(_, EnumPosition::Name(name))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Name(name)) => {
            find_where_used_as_top_name(&ctx, name)
                .into_iter()
                .chain(find_where_used_as_type(ctx, name))
                .collect()
        }

        SchemaPosition::DataSource(_, SourcePosition::Name(name)) => find_where_used_for_native_type(&ctx, name),

        // Fields
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Type(r#type)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Type(r#type))) => {
            find_where_used_as_top_name(&ctx, r#type)
                .into_iter()
                .chain(find_where_used_as_type(ctx, r#type))
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
            Some("fields") => find_where_used_in_model(&ctx, arg_value.as_str(), model_id, ctx.initiating_file_id),
            Some("references") => {
                info!("finding the field from references");

                let (_, field) = ctx.db.ast(ctx.initiating_file_id)[model_id]
                    .iter_fields()
                    .find(|(id, _)| id == &field_id)
                    .unwrap();

                let referenced_model = field.field_type.name();

                let Some(ref_model_id) = ctx.db.find_model(referenced_model) else {
                    warn!("Could not find model with name: {}", referenced_model);
                    return empty_references();
                };

                find_where_used_in_model(&ctx, arg_value.as_str(), ref_model_id.id.1, ref_model_id.id.0)
            }
            _ => empty_references(),
        },

        // ? This might make more sense to add as a definition rather than a reference
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Attribute(name, _, _)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Attribute(name, _, _))) => {
            if let Some(ds) = ctx.datasource() {
                if name.contains(ds.name.as_str()) {
                    return find_where_used_as_top_name(&ctx, ds.name.as_str());
                }
            }
            empty_references()
        }

        SchemaPosition::Model(
            model_id,
            ModelPosition::ModelAttribute(_attr_name, _, AttributePosition::ArgumentValue(_, arg_val)),
        ) => find_where_used_in_model(&ctx, arg_val.as_str(), model_id, ctx.initiating_file_id),

        _ => empty_references(),
    }
}

fn find_where_used_in_model(
    ctx: &ReferencesContext<'_>,
    name: &str,
    model_id: ModelId,
    file_id: FileId,
) -> Vec<Location> {
    let Some(model) = ctx
        .db
        .walk_models()
        .chain(ctx.db.walk_views())
        .find(|model| model.id.1 == model_id && model.file_id() == file_id)
    else {
        warn!("Could not find model");
        return empty_references();
    };

    let identifier = if let Some(field) = model.scalar_fields().find(|field| field.name() == name) {
        field.ast_field().identifier()
    } else {
        warn!("Could not find field with name: `{}`", name);
        return empty_references();
    };

    identifiers_to_locations(vec![identifier], ctx)
}

fn find_where_used_for_native_type(ctx: &ReferencesContext<'_>, name: &str) -> Vec<Location> {
    info!("Get references for native types");

    let references = ctx
        .db
        .walk_tops()
        .flat_map(|top| match top.ast_top() {
            Top::CompositeType(composite_type) => find_native_type_locations(ctx, name, composite_type.iter_fields()),
            Top::Model(model) => find_native_type_locations(ctx, name, model.iter_fields()),

            Top::Enum(_) | Top::Source(_) | Top::Generator(_) => empty_references(),
        })
        .collect();

    references
}

fn find_native_type_locations<'a>(
    ctx: &ReferencesContext<'_>,
    name: &str,
    fields: impl Iterator<Item = (FieldId, &'a Field)>,
) -> Vec<Location> {
    let identifiers = fields
        .filter_map(|field| {
            field
                .1
                .attributes
                .iter()
                .find(|attr| extract_ds_from_native_type(attr.name()) == name)
                .map(|attr| attr.identifier())
        })
        .collect();

    identifiers_to_locations(identifiers, ctx)
}

fn extract_ds_from_native_type(attr_name: &str) -> &str {
    let split = attr_name.split('.').collect::<Vec<&str>>()[0];
    split
}

fn find_where_used_as_type(ctx: ReferencesContext<'_>, name: &str) -> Vec<Location> {
    info!("Get references for top");

    let references: Vec<Location> = ctx
        .db
        .walk_tops()
        .flat_map(|top| match top.ast_top() {
            Top::Model(model) => {
                info!("Get references in model");

                let fields = model.iter_fields();

                let identifiers = get_relevent_identifiers(fields, name);

                identifiers_to_locations(identifiers, &ctx)
            }
            Top::CompositeType(composite_type) => {
                info!("Get references in composite type");

                let fields = composite_type.iter_fields();
                let ids = get_relevent_identifiers(fields, name);

                identifiers_to_locations(ids, &ctx)
            }
            Top::Enum(_) | Top::Source(_) | Top::Generator(_) => empty_references(),
        })
        .collect();

    references
}

fn find_where_used_as_top_name(ctx: &ReferencesContext<'_>, name: &str) -> Vec<Location> {
    fn ident_to_location(id: &Identifier, name: &str, ctx: &ReferencesContext<'_>) -> Vec<Location> {
        if id.name == name {
            identifiers_to_locations(vec![id], ctx)
        } else {
            empty_references()
        }
    }

    let references: Vec<Location> = ctx
        .db
        .walk_tops()
        .flat_map(|top| match top.ast_top() {
            Top::CompositeType(composite_type) => ident_to_location(composite_type.identifier(), name, ctx),

            Top::Enum(enm) => ident_to_location(enm.identifier(), name, ctx),

            Top::Model(model) => ident_to_location(model.identifier(), name, ctx),

            Top::Source(source) => ident_to_location(source.identifier(), name, ctx),

            Top::Generator(_) => empty_references(),
        })
        .collect();

    references
}

fn get_relevent_identifiers<'a, 'b>(
    fields: impl Iterator<Item = (FieldId, &'a Field)>,
    name: &str,
) -> Vec<&'a Identifier> {
    fields
        .filter_map(|(_id, field)| match &field.field_type {
            FieldType::Supported(id) => {
                if id.name == name {
                    Some(id)
                } else {
                    None
                }
            }
            FieldType::Unsupported(_, _) => None,
        })
        .collect()
}

fn identifiers_to_locations(ids: Vec<&Identifier>, ctx: &ReferencesContext<'_>) -> Vec<Location> {
    ids.iter()
        .filter_map(|identifier| {
            let file_id = identifier.span.file_id;

            let source = ctx.db.source(file_id);
            let range = span_to_range(identifier.span, source);

            let file_name = ctx.db.file_name(file_id);

            let uri = if let Ok(uri) = Url::parse(file_name) {
                uri
            } else {
                warn!("Could not find file at path: {:?}", file_name);
                return None;
            };

            Some(Location { uri, range })
        })
        .collect()
}
