use log::*;
use lsp_types::{Location, ReferenceParams, Url};
use psl::{
    diagnostics::Span, error_tolerant_parse_configuration, parser_database::ParserDatabase,
    schema_ast::ast::SchemaPosition, Diagnostics, SourceFile,
};

use crate::{
    find_where_used,
    offsets::{position_to_offset, span_to_range},
    LSPContext,
};

pub(super) type ReferencesContext<'a> = LSPContext<'a, ReferenceParams>;

pub(crate) fn empty_references() -> Vec<Location> {
    Vec::new()
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

    find_locations_for_target(ctx, target_position)
}

fn find_locations_for_target(ctx: ReferencesContext<'_>, target: SchemaPosition) -> Vec<Location> {
    find_where_used::reference_locations_for_target(&ctx, target)
        .into_iter()
        .filter_map(|span| span_to_location(span, &ctx))
        .collect()
}

fn span_to_location(span: Span, ctx: &ReferencesContext<'_>) -> Option<Location> {
    let file_id = span.file_id;

    let source = ctx.db.source(file_id);
    let range = span_to_range(span, source);
    let file_name = ctx.db.file_name(file_id);

    let uri = if let Ok(uri) = Url::parse(file_name) {
        uri
    } else {
        return None;
    };

    Some(Location { uri, range })
}
