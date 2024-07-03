use log::{info, warn};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::{walkers::Walker, ParserDatabase, RelationFieldId},
    schema_ast::ast::{self, Field, FieldPosition, Model, ModelPosition, WithDocumentation, WithName},
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
            let initiating_model = &ctx.db.ast(ctx.initiating_file_id)[model_id];
            let initiating_field = &initiating_model[field_id];

            let Some(target_top) = ctx.db.find_top(name).map(|top| top.ast_top()) else {
                warn!("Couldn't find a block called {}", name);
                return None;
            };

            let relation_info = match target_top.get_type() {
                "model" | "view" => {
                    let target_model = target_top.as_model().unwrap();
                    get_relation_info(&ctx, initiating_model, target_model, initiating_field)
                }
                _ => None,
            };

            Some(format_hover_content(
                target_top.documentation().unwrap_or(""),
                target_top.get_type(),
                name,
                relation_info,
            ))
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

fn get_relation_info<'a>(
    ctx: &'a LSPContext<'a, HoverParams>,
    model: &Model,
    target_model: &'a ast::Model,
    field: &'a Field,
) -> Option<(String, &'a Field)> {
    ctx.db.walk_relations().find_map(|relation| {
        let [referencing_model, referenced_model] = relation.models();
        let fields = relation.relation_fields().collect::<Vec<Walker<RelationFieldId>>>();

        let referencing_name = ctx.db.ast(referencing_model.0)[referencing_model.1].name();
        let referenced_name = ctx.db.ast(referenced_model.0)[referenced_model.1].name();

        let self_relation = if relation.is_self_relation() { " on self" } else { " " };
        let relation_kind = format!("{}{}", relation.relation_kind(), self_relation);

        let field = if referencing_name == model.name() && referenced_name == target_model.name() {
            if relation.is_self_relation() && fields[1].ast_field().name() == field.name() {
                fields[0]
            } else {
                fields[1]
            }
            // * (@druue) Flipped for one-to-many relations when accessing from the block that has the many relation.
            // ```
            // model ModelNameA {
            //    id  Int        @id
            //    bId Int
            //    val ModelNameB @relation(fields: [bId], references: [id])
            // }
            //
            // model ModelNameB {
            //   id Int          @id
            //   A  ModelNameA[]
            //      ^^^^^^^^^^ // When accessing from here
            // }
            // ```
            // * I have no idea why.
        } else if referencing_name == target_model.name() && referenced_name == model.name() {
            fields[0]
        } else {
            warn!("couldn't find relation");
            return None;
        };

        Some((relation_kind, field.ast_field()))
    })
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
