use log::info;
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::ParserDatabase,
    schema_ast::ast::{self, FieldPosition},
    Diagnostics, SourceFile,
};

use crate::LSPContext;

pub(super) type HoverContext<'a> = LSPContext<'a, HoverParams>;

impl<'a> HoverContext<'a> {
    pub(super) fn position(&self) -> Option<usize> {
        let pos = self.params.text_document_position_params.position;
        let initiating_doc = self.initiating_file_source();

        super::position_to_offset(&pos, initiating_doc)
    }
}

pub fn run(schema_files: Vec<(String, SourceFile)>, params: HoverParams) -> Hover {
    let (_, config, _) = error_tolerant_parse_configuration(&schema_files);

    let db = {
        let mut diag = Diagnostics::new();
        ParserDatabase::new(&schema_files, &mut diag)
    };

    let Some(initiating_file_id) = db.file_id(params.text_document_position_params.text_document.uri.as_str()) else {
        info!("Initiating file name is not found in the schema");
        panic!("Initiating file name is not found in the schema");
    };

    let ctx = HoverContext {
        db: &db,
        config: &config,
        initiating_file_id,
        params: &params,
    };

    hover(ctx)
}

fn hover(ctx: HoverContext<'_>) -> Hover {
    let position = match ctx.position() {
        Some(pos) => pos,
        None => {
            info!("Received a position outside of the document boundaries in CompletionParams");
            panic!("Received a position outside of the document boundaries in CompletionParams")
        }
    };

    let ast = ctx.db.ast(ctx.initiating_file_id);
    let contents = match ast.find_at_position(position) {
        psl::schema_ast::ast::SchemaPosition::TopLevel => {
            format_hover_content("documentation", "top_variant", "top_name")
        }
        psl::schema_ast::ast::SchemaPosition::Model(_model_id, model_position) => {
            info!("We're inside a model");
            info!("We are here: {:?}", model_position);

            let name = match model_position {
                ast::ModelPosition::Name(name) => name,
                ast::ModelPosition::Field(_, FieldPosition::Type(name)) => name,
                _ => todo!(),
            };

            let top = ast.iter_tops().find(|(_, top)| top.name() == name);

            let doc = top.and_then(|(_, top)| top.documentation()).unwrap_or("");

            format_hover_content(doc, "model", name)
        }
        psl::schema_ast::ast::SchemaPosition::Enum(_enum_id, enum_position) => {
            info!("We are here: {:?}", enum_position);
            format_hover_content("documentation", "top_variant", "top_name")
        }
        psl::schema_ast::ast::SchemaPosition::DataSource(_ds_id, source_position) => {
            info!("We are here: {:?}", source_position);
            format_hover_content("documentation", "top_variant", "top_name")
        }
    };

    Hover { contents, range: None }
}

fn format_hover_content(documentation: &str, top_variant: &str, top_name: &str) -> HoverContents {
    let full_signature = format!("```prisma\n{top_variant} {top_name} {{}}\n```\n___\n{documentation}");

    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: full_signature,
    })
}
