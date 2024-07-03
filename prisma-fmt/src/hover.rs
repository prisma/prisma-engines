use log::{info, warn};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    error_tolerant_parse_configuration,
    parser_database::{walkers::Walker, ParserDatabase, RelationFieldId},
    schema_ast::ast::{self, Field, FieldPosition, WithName},
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
        psl::schema_ast::ast::SchemaPosition::Model(model_id, model_position) => {
            let Some(model) = ctx
                .db
                .walk_models()
                .chain(ctx.db.walk_views())
                .find(|model| model.id.1 == model_id)
            else {
                warn!("This shouldn't be possible");
                return None;
            };

            let (name, relation) = match model_position {
                ast::ModelPosition::Name(name) => (name, None),
                ast::ModelPosition::Field(_, FieldPosition::Type(name)) => {
                    let target_model = ctx.db.walk_models().chain(ctx.db.walk_views()).find_map(|model| {
                        if model.ast_model().name() == name {
                            Some(model.ast_model())
                        } else {
                            None
                        }
                    });

                    let Some(target_model) = target_model else {
                        warn!("Could not find model with name: {:?}", name);
                        return None;
                    };

                    let relation_kind = ctx.db.walk_relations().find_map(|relation| {
                        let [referencing_model, referenced_model] = relation.models();

                        relation
                            .models()
                            .iter()
                            .for_each(|(fid, mid)| info!("{}", &ctx.db.ast(*fid)[*mid].name()));

                        let referencing_name = ctx.db.ast(referencing_model.0)[referencing_model.1].name();
                        let referenced_name = ctx.db.ast(referenced_model.0)[referenced_model.1].name();

                        info!(
                            "referencing from: {} referenced: {}, target name: {}",
                            referencing_name,
                            referenced_name,
                            target_model.name()
                        );

                        let self_relation = if relation.is_self_relation() { " on self" } else { " " };
                        let relation_kind = format!("{}{}", relation.relation_kind(), self_relation);
                        dbg!(&relation_kind);

                        let field = if referencing_name == model.ast_model().name()
                            && referenced_name == target_model.name()
                        {
                            relation.relation_fields().collect::<Vec<Walker<RelationFieldId>>>()[1]
                        } else if referencing_name == target_model.name() && referenced_name == model.ast_model().name()
                        {
                            relation.relation_fields().collect::<Vec<Walker<RelationFieldId>>>()[0]
                        } else {
                            return None;
                        };

                        Some((relation_kind, field.ast_field()))
                    });

                    (name, relation_kind)
                }
                _ => ("", None),
            };

            let top = ctx.db.find_top(name).map(|top| top.ast_top());

            let (variant, doc) = match top {
                Some(top) => {
                    let doc = top.documentation().unwrap_or("");
                    (top.get_type(), doc)
                }
                None => ("", ""),
            };

            Some(format_hover_content(doc, variant, name, relation))
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
