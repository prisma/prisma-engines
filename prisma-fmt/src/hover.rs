use log::{info, warn};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::ParserDatabase,
    schema_ast::ast::{self, Field, FieldPosition, ModelPosition, WithDocumentation},
    Diagnostics, SourceFile,
};

use crate::{offsets::position_to_offset, LSPContext};

pub(super) type HoverContext<'a> = LSPContext<'a, HoverParams>;

impl<'a> HoverContext<'a> {
    pub(super) fn position(&self) -> Option<usize> {
        let pos = self.params.text_document_position_params.position;
        let initiating_doc = self.initiating_file_source();

        position_to_offset(&pos, initiating_doc)
    }
}

pub fn run(schema_files: Vec<(String, SourceFile)>, params: HoverParams) -> Option<Hover> {
    let (_, config, _) = error_tolerant_parse_configuration(&schema_files);

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(&schema_files, &mut diag)
    };

    let Some(initiating_file_id) = db.file_id(params.text_document_position_params.text_document.uri.as_str()) else {
        warn!("Initiating file name is not found in the schema");
        return None;
    };

    let ctx = HoverContext {
        db: &db,
        config: &config,
        initiating_file_id,
        params: &params,
    };

    hover(ctx)
}

fn hover(ctx: HoverContext<'_>) -> Option<Hover> {
    info!("Calling Hover");
    let position = match ctx.position() {
        Some(pos) => pos,
        None => {
            warn!("Received a position outside of the document boundaries in HoverParams");
            return None;
        }
    };

    let ast = ctx.db.ast(ctx.initiating_file_id);
    let contents = match ast.find_at_position(position) {
        psl::schema_ast::ast::SchemaPosition::TopLevel => None,
        psl::schema_ast::ast::SchemaPosition::Model(model_id, ModelPosition::Name(name)) => {
            let walker = ctx.db.walk((ctx.initiating_file_id, model_id));
            let model = walker.ast_model();
            let variant = if model.is_view() { "view" } else { "model" };

            Some(format_hover_content(
                model.documentation().unwrap_or(""),
                variant,
                name,
                None,
            ))
        }

        psl::schema_ast::ast::SchemaPosition::Model(
            model_id,
            ModelPosition::Field(field_id, FieldPosition::Type(name)),
        ) => {
            let initiating_field = &ctx.db.walk((ctx.initiating_file_id, model_id)).field(field_id);

            match initiating_field.refine() {
                psl::parser_database::walkers::RefinedFieldWalker::Scalar(_) => None,
                psl::parser_database::walkers::RefinedFieldWalker::Relation(rf) => {
                    let relation = rf.relation();
                    let opposite_model = rf.related_model();
                    let opposite_field = rf.opposite_relation_field().unwrap().ast_field();
                    let related_model_type = if opposite_model.ast_model().is_view() {
                        "view"
                    } else {
                        "model"
                    };
                    let self_relation = if relation.is_self_relation() { " on self" } else { " " };
                    let relation_kind = format!("{}{}", relation.relation_kind(), self_relation);

                    Some(format_hover_content(
                        opposite_model.ast_model().documentation().unwrap_or_default(),
                        related_model_type,
                        name,
                        Some((relation_kind, opposite_field)),
                    ))
                }
            }
        }
        ast::SchemaPosition::Model(_, model_position) => {
            info!("We are here {:?}", model_position);
            None
        }
        psl::schema_ast::ast::SchemaPosition::CompositeType(_composite_id, composite_positon) => {
            info!("We are here: {:?}", composite_positon);
            None
        }
        psl::schema_ast::ast::SchemaPosition::Enum(_enum_id, enum_position) => {
            info!("We are here: {:?}", enum_position);
            None
        }
        psl::schema_ast::ast::SchemaPosition::DataSource(_ds_id, source_position) => {
            info!("We are here: {:?}", source_position);
            None
        }
        psl::schema_ast::ast::SchemaPosition::Generator(_gen_id, gen_position) => {
            info!("We are here: {:?}", gen_position);
            None
        }
    };

    contents.map(|contents| Hover { contents, range: None })
}

fn format_hover_content(
    documentation: &str,
    variant: &str,
    top_name: &str,
    relation: Option<(String, &Field)>,
) -> HoverContents {
    let fancy_line_break = String::from("\n___\n");
    let (field, relation_kind) = relation.map_or((Default::default(), Default::default()), |(rk, field)| {
        (format!("\n\t...\n\t{field}\n"), format!("{rk}{fancy_line_break}"))
    });
    let prisma_display = match variant {
        "model" | "enum" | "view" | "composite type" => {
            format!("```prisma\n{variant} {top_name} {{{field}}}\n```{fancy_line_break}{relation_kind}")
        }
        _ => "".to_owned(),
    };
    let full_signature = format!("{prisma_display}{documentation}");

    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: full_signature,
    })
}
