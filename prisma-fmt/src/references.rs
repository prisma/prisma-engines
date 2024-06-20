use log::*;
use lsp_types::{Location, ReferenceParams, Url};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::ParserDatabase,
    schema_ast::ast::{
        EnumPosition, Field, FieldId, FieldPosition, FieldType, Identifier, ModelPosition, SchemaPosition,
        SourcePosition, Top, WithIdentifier,
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

    get_reference_target(ctx, target_position)
}

fn get_reference_target(ctx: ReferencesContext<'_>, target: SchemaPosition) -> Vec<Location> {
    info!("{:?}", target);

    match target {
        // Block Names
        SchemaPosition::Model(_, ModelPosition::Name(name)) => find_where_used_as_top_name(&ctx, name)
            .into_iter()
            .chain(find_where_used_as_type(ctx, name))
            .collect(),

        SchemaPosition::Enum(_, EnumPosition::Name(name)) => find_where_used_as_type(ctx, name),
        SchemaPosition::DataSource(_, SourcePosition::Name(_name)) => empty_references(),
        // Fields
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Type(r#type))) => {
            find_where_used_as_top_name(&ctx, r#type)
                .into_iter()
                .chain(find_where_used_as_type(ctx, r#type))
                .collect()
        }

        _ => empty_references(),
    }
}

fn find_where_used_as_type(ctx: ReferencesContext<'_>, name: &str) -> Vec<Location> {
    info!("Get references for top");

    let references: Vec<Location> = ctx
        .db
        .iter_tops()
        .map(|(file_id, _top_id, top)| match top {
            Top::Model(model) => {
                info!("Get references in model");
                info!("{:?}", file_id);

                let fields = model.iter_fields();

                let identifiers = get_relevent_identifiers(fields, name);

                identifiers_to_locations(identifiers, &ctx)
            }
            Top::CompositeType(composite_type) => {
                info!("Get references in composite type");
                info!("{:?}", file_id);

                let fields = composite_type.iter_fields();
                let ids = get_relevent_identifiers(fields, name);

                identifiers_to_locations(ids, &ctx)
            }
            Top::Enum(_) | Top::Source(_) | Top::Generator(_) => empty_references(),
        })
        .flatten()
        .collect();

    references
}

// fn find_where_used_in_relation(ctx: &ReferencesContext<'_>, name: &str) -> Vec<Location> {
//     empty_references()
// }

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
        .iter_tops()
        .map(|(_file_id, _top_id, top)| match top {
            Top::CompositeType(composite_type) => ident_to_location(composite_type.identifier(), name, ctx),

            Top::Enum(enm) => ident_to_location(enm.identifier(), name, ctx),

            Top::Model(model) => ident_to_location(model.identifier(), name, ctx),

            Top::Source(source) => ident_to_location(source.identifier(), name, ctx),

            Top::Generator(_) => empty_references(),
        })
        .flatten()
        .collect();

    references
}

fn get_relevent_identifiers<'a, 'b>(
    fields: impl Iterator<Item = (FieldId, &'a Field)> + std::iter::ExactSizeIterator,
    name: &str,
) -> Vec<&'a Identifier> {
    let filter_map = fields
        .filter_map(|(_id, field)| {
            info!("{:?}", field);
            match &field.field_type {
                FieldType::Supported(id) => {
                    if id.name == name {
                        Some(id)
                    } else {
                        None
                    }
                }
                FieldType::Unsupported(_, _) => None,
            }
        })
        .collect::<Vec<&Identifier>>();

    filter_map
}

fn identifiers_to_locations<'a>(ids: Vec<&'a Identifier>, ctx: &ReferencesContext<'_>) -> Vec<Location> {
    ids.iter()
        .filter_map(|identifier| {
            let file_id = identifier.span.file_id;

            let source = ctx.db.source(file_id);
            let range = span_to_range(identifier.span, source);

            let file_name = ctx.db.file_name(file_id);

            let uri = if let Ok(uri) = Url::parse(file_name) {
                uri
            } else {
                warn!("Failed to parse file path: {:?}", file_name);
                return None;
            };

            Some(Location { uri, range })
        })
        .collect()
}
