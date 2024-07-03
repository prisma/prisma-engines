use log::{info, warn};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::ParserDatabase,
    schema_ast::ast::{self, FieldPosition},
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
            warn!("Received a position outside of the document boundaries in CompletionParams");
            return None;
        }
    };

    let ast = ctx.db.ast(ctx.initiating_file_id);
    let contents = match ast.find_at_position(position) {
        psl::schema_ast::ast::SchemaPosition::TopLevel => None,
        psl::schema_ast::ast::SchemaPosition::Model(_model_id, model_position) => {
            let name = match model_position {
                ast::ModelPosition::Name(name) => name,
                ast::ModelPosition::Field(_, FieldPosition::Type(name)) => name,
                _ => "",
            };

            let top = ctx.db.walk_tops().find_map(|top| {
                if top.ast_top().name() == name {
                    Some(top.ast_top())
                } else {
                    None
                }
            });

            let (variant, doc) = match top {
                Some(top) => {
                    let doc = top.documentation().unwrap_or("");
                    (top.get_type(), doc)
                }
                None => ("", ""),
            };

            Some(format_hover_content(doc, variant, name))
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

    match contents {
        Some(contents) => Some(Hover { contents, range: None }),
        None => None,
    }
}

fn format_hover_content(documentation: &str, variant: &str, top_name: &str) -> HoverContents {
    let fancy_line_break = String::from("\n___\n");
    let prisma_display = match variant {
        "model" | "enum" | "view" | "composite type" => {
            format!("```prisma\n{variant} {top_name} {{}}\n```{fancy_line_break}")
        }
        _ => "".to_owned(),
    };
    let full_signature = format!("{prisma_display}{documentation}");

    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: full_signature,
    })
}
