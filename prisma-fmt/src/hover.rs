use log::warn;
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use psl::{
    Diagnostics, SourceFile, error_tolerant_parse_configuration,
    parser_database::{
        NoExtensionTypes, ParserDatabase, RelationFieldId, ScalarFieldType,
        walkers::{self, Walker},
    },
    schema_ast::ast::{
        self, CompositeTypePosition, EnumPosition, EnumValuePosition, Field, FieldPosition, ModelPosition,
        SchemaPosition, WithDocumentation, WithName,
    },
};

use crate::{LSPContext, offsets::position_to_offset};

pub(super) type HoverContext<'a> = LSPContext<'a, HoverParams>;

impl HoverContext<'_> {
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
        ParserDatabase::new(&schema_files, &mut diag, &NoExtensionTypes)
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
    let position = match ctx.position() {
        Some(pos) => pos,
        None => {
            warn!("Received a position outside of the document boundaries in HoverParams");
            return None;
        }
    };

    let ast = ctx.db.ast(ctx.initiating_file_id);
    let contents = match ast.find_at_position(position) {
        SchemaPosition::TopLevel => None,

        // --- Block Names ---
        SchemaPosition::Model(model_id, ModelPosition::Name(name)) => {
            let model = ctx.db.walk((ctx.initiating_file_id, model_id)).ast_model();
            let variant = if model.is_view() { "view" } else { "model" };

            Some(format_hover_content(
                model.documentation().unwrap_or(""),
                variant,
                name,
                None,
            ))
        }
        SchemaPosition::Enum(enum_id, EnumPosition::Name(name)) => {
            let enm = ctx.db.walk((ctx.initiating_file_id, enum_id)).ast_enum();
            Some(hover_enum(enm, name))
        }
        SchemaPosition::CompositeType(ct_id, CompositeTypePosition::Name(name)) => {
            let ct = ctx.db.walk((ctx.initiating_file_id, ct_id)).ast_composite_type();
            Some(hover_composite(ct, name))
        }

        // --- Block Field Names ---
        SchemaPosition::Model(model_id, ModelPosition::Field(field_id, FieldPosition::Name(name))) => {
            let field = ctx
                .db
                .walk((ctx.initiating_file_id, model_id))
                .field(field_id)
                .ast_field();

            Some(format_hover_content(
                field.documentation().unwrap_or_default(),
                "field",
                name,
                None,
            ))
        }
        SchemaPosition::CompositeType(ct_id, CompositeTypePosition::Field(field_id, FieldPosition::Name(name))) => {
            let field = ctx.db.walk((ctx.initiating_file_id, ct_id)).field(field_id).ast_field();

            Some(format_hover_content(
                field.documentation().unwrap_or_default(),
                "field",
                name,
                None,
            ))
        }
        SchemaPosition::Enum(enm_id, EnumPosition::Value(value_id, EnumValuePosition::Name(name))) => {
            let value = ctx
                .db
                .walk((ctx.initiating_file_id, enm_id))
                .value(value_id)
                .ast_value();

            Some(format_hover_content(
                value.documentation().unwrap_or_default(),
                "value",
                name,
                None,
            ))
        }

        // --- Block Field Types ---
        SchemaPosition::Model(model_id, ModelPosition::Field(field_id, FieldPosition::Type(name))) => {
            let initiating_field = &ctx.db.walk((ctx.initiating_file_id, model_id)).field(field_id);

            initiating_field.refine().and_then(|field| match field {
                walkers::RefinedFieldWalker::Scalar(scalar) => match scalar.scalar_field_type() {
                    ScalarFieldType::CompositeType(_) => {
                        let ct = scalar.field_type_as_composite_type().unwrap().ast_composite_type();
                        Some(hover_composite(ct, ct.name()))
                    }
                    ScalarFieldType::Enum(_) => {
                        let enm = scalar.field_type_as_enum().unwrap().ast_enum();
                        Some(hover_enum(enm, enm.name()))
                    }
                    _ => None,
                },
                walkers::RefinedFieldWalker::Relation(rf) => {
                    let opposite_model = rf.related_model();
                    let relation_info = rf.opposite_relation_field().map(|rf| (rf, rf.ast_field()));
                    let related_model_type = if opposite_model.ast_model().is_view() {
                        "view"
                    } else {
                        "model"
                    };

                    Some(format_hover_content(
                        opposite_model.ast_model().documentation().unwrap_or_default(),
                        related_model_type,
                        name,
                        relation_info,
                    ))
                }
            })
        }

        SchemaPosition::CompositeType(ct_id, CompositeTypePosition::Field(field_id, FieldPosition::Type(_))) => {
            let field = &ctx.db.walk((ctx.initiating_file_id, ct_id)).field(field_id);
            match field.r#type() {
                psl::parser_database::ScalarFieldType::CompositeType(_) => {
                    let ct = field.field_type_as_composite_type().unwrap().ast_composite_type();
                    Some(hover_composite(ct, ct.name()))
                }
                psl::parser_database::ScalarFieldType::Enum(_) => {
                    let enm = field.field_type_as_enum().unwrap().ast_enum();
                    Some(hover_enum(enm, enm.name()))
                }
                _ => None,
            }
        }
        _ => None,
    };

    contents.map(|contents| Hover { contents, range: None })
}

fn hover_enum(enm: &ast::Enum, name: &str) -> HoverContents {
    format_hover_content(enm.documentation().unwrap_or_default(), "enum", name, None)
}

fn hover_composite(ct: &ast::CompositeType, name: &str) -> HoverContents {
    format_hover_content(ct.documentation().unwrap_or_default(), "type", name, None)
}

fn format_hover_content(
    documentation: &str,
    variant: &str,
    name: &str,
    relation: Option<(Walker<RelationFieldId>, &Field)>,
) -> HoverContents {
    let fancy_line_break = String::from("\n___\n");

    let (field, relation_kind) = format_relation_info(relation, &fancy_line_break);

    let prisma_display = match variant {
        "model" | "enum" | "view" | "type" => {
            format!("```prisma\n{variant} {name} {{{field}}}\n```{fancy_line_break}{relation_kind}")
        }
        "field" | "value" => format!("```prisma\n{name}\n```{fancy_line_break}"),
        _ => "".to_owned(),
    };
    let full_signature = format!("{prisma_display}{documentation}");

    HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: full_signature,
    })
}

fn format_relation_info(
    relation: Option<(Walker<RelationFieldId>, &Field)>,
    fancy_line_break: &String,
) -> (String, String) {
    if let Some((rf, field)) = relation {
        let relation = rf.relation();

        let fields = rf
            .referencing_fields()
            .map(|fields| fields.map(|f| f.to_string()).collect::<Vec<String>>().join(", "))
            .map_or_else(String::new, |fields| format!(", fields: [{fields}]"));

        let references = rf
            .referenced_fields()
            .map(|fields| fields.map(|f| f.to_string()).collect::<Vec<String>>().join(", "))
            .map_or_else(String::new, |fields| format!(", references: [{fields}]"));

        let self_relation = if relation.is_self_relation() { " on self" } else { "" };
        let relation_kind = format!("{}{}", relation.relation_kind(), self_relation);

        let relation_name = relation.relation_name();
        let relation_inner = format!("name: \"{relation_name}\"{fields}{references}");

        (
            format!("\n\t...\n\t{field} @relation({relation_inner})\n"),
            format!("{relation_kind}{fancy_line_break}"),
        )
    } else {
        ("".to_owned(), "".to_owned())
    }
}
