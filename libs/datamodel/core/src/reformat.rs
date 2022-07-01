use crate::ParserDatabase;
use parser_database::walkers;
use schema_ast::{ast, source_file::SourceFile};
use std::{borrow::Cow, sync::Arc};

/// Returns either the reformatted schema, or the original input if we can't reformat. This happens
/// if and only if the source does not parse to a well formed AST.
pub fn reformat(source: &str, indent_width: usize) -> Option<String> {
    let file = SourceFile::new_allocated(Arc::from(source.to_owned().into_boxed_str()));

    let mut diagnostics = diagnostics::Diagnostics::new();
    let db = parser_database::ParserDatabase::new(file, &mut diagnostics);

    let source_to_reformat = if diagnostics.has_errors() {
        Cow::Borrowed(source)
    } else {
        let mut missing_bits = Vec::new();
        let mut ctx = MagicReformatCtx {
            original_schema: source,
            missing_bits: &mut missing_bits,
            db: &db,
        };
        push_missing_fields(&mut ctx);
        push_missing_attributes(&mut ctx);
        push_missing_relation_attribute_args(&mut ctx);
        missing_bits.sort_by_key(|bit| bit.position);

        if missing_bits.is_empty() {
            Cow::Borrowed(source)
        } else {
            Cow::Owned(enrich(source, &missing_bits))
        }
    };

    schema_ast::reformat(&source_to_reformat, indent_width)
}

struct MagicReformatCtx<'a> {
    original_schema: &'a str,
    missing_bits: &'a mut Vec<MissingBit>,
    db: &'a ParserDatabase,
}

fn enrich(input: &str, missing_bits: &[MissingBit]) -> String {
    let bits = missing_bits.iter().scan(0usize, |last_insert_position, missing_bit| {
        let start: usize = *last_insert_position;
        *last_insert_position = missing_bit.position;

        Some((start, missing_bit.position, &missing_bit.content))
    });

    let mut out = String::with_capacity(input.len() + missing_bits.iter().map(|mb| mb.content.len()).sum::<usize>());

    for (start, end, insert_content) in bits {
        out.push_str(&input[start..end]);
        out.push_str(insert_content);
    }

    let last_span_start = missing_bits.last().map(|b| b.position).unwrap_or(0);
    out.push_str(&input[last_span_start..]);

    out
}

#[derive(Debug)]
struct MissingBit {
    position: usize,
    content: String,
}

fn push_missing_relation_attribute_args(ctx: &mut MagicReformatCtx<'_>) {
    for relation in ctx.db.walk_relations() {
        match relation.refine() {
            walkers::RefinedRelationWalker::Inline(inline_relation) => {
                push_inline_relation_missing_arguments(inline_relation, ctx)
            }
            walkers::RefinedRelationWalker::ImplicitManyToMany(_) => (),
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => (),
        }
    }
}

fn push_inline_relation_missing_arguments(
    inline_relation: walkers::InlineRelationWalker<'_>,
    ctx: &mut MagicReformatCtx<'_>,
) {
    if let Some(forward) = inline_relation.forward_relation_field() {
        let relation_attribute = if let Some(attr) = forward.relation_attribute() {
            attr
        } else {
            return;
        };

        let mut extra_args = Vec::new();

        if inline_relation.referencing_fields().is_none() {
            extra_args.push(fields_argument(inline_relation));
        }

        if forward.referenced_fields().is_none() {
            extra_args.push(references_argument(inline_relation));
        }

        let extra_args = extra_args.join(", ");

        let (prefix, suffix, position) = if relation_attribute.arguments.arguments.is_empty() {
            ("(", ")", relation_attribute.span.end)
        } else {
            (", ", "", relation_attribute.span.end - 1)
        };

        ctx.missing_bits.push(MissingBit {
            position,
            content: format!("{prefix}{extra_args}{suffix}"),
        });
    }
}

fn push_missing_attributes(ctx: &mut MagicReformatCtx<'_>) {
    for relation in ctx.db.walk_relations() {
        if let walkers::RefinedRelationWalker::Inline(inline_relation) = relation.refine() {
            push_missing_relation_attribute(inline_relation, ctx);
        }
    }
}

fn push_missing_relation_attribute(inline_relation: walkers::InlineRelationWalker<'_>, ctx: &mut MagicReformatCtx<'_>) {
    if let Some(forward) = inline_relation.forward_relation_field() {
        if forward.relation_attribute().is_some() {
            return;
        }

        let mut content = String::from(" @relation(");
        content.push_str(&fields_argument(inline_relation));
        content.push_str(", ");
        content.push_str(&references_argument(inline_relation));
        content.push(')');

        ctx.missing_bits.push(MissingBit {
            position: before_newline(forward.ast_field().span.end, ctx.original_schema),
            content,
        })
    }
}

// this finds all auto generated fields, that are added during auto generation AND are missing from the original input.
fn push_missing_fields(ctx: &mut MagicReformatCtx<'_>) {
    for relation in ctx.db.walk_relations() {
        if let Some(inline) = relation.refine().as_inline() {
            push_missing_fields_for_relation(inline, ctx);
        }
    }
}

fn push_missing_fields_for_relation(relation: walkers::InlineRelationWalker<'_>, ctx: &mut MagicReformatCtx<'_>) {
    push_missing_relation_fields(relation, ctx);
    push_missing_scalar_fields(relation, ctx);
}

fn push_missing_relation_fields(inline: walkers::InlineRelationWalker<'_>, ctx: &mut MagicReformatCtx<'_>) {
    if inline.back_relation_field().is_none() {
        let referencing_model_name = inline.referencing_model().name();
        let ignore = if inline.referencing_model().is_ignored() {
            "@ignore"
        } else {
            ""
        };
        let arity = if inline.is_one_to_one() { "?" } else { "[]" };

        ctx.missing_bits.push(MissingBit {
            position: inline.referenced_model().ast_model().span.end - 1,
            content: format!("{referencing_model_name} {referencing_model_name}{arity} {ignore}\n"),
        });
    }

    if inline.forward_relation_field().is_none() {
        let field_name = inline.referenced_model().name();
        let field_type = field_name;
        let arity = render_arity(forward_relation_field_arity(inline));
        let fields_arg = fields_argument(inline);
        let references_arg = references_argument(inline);
        ctx.missing_bits.push(MissingBit {
            position: inline.referencing_model().ast_model().span.end - 1,
            content: format!("{field_name} {field_type}{arity} @relation({fields_arg}, {references_arg})\n"),
        })
    }
}

fn push_missing_scalar_fields(inline: walkers::InlineRelationWalker<'_>, ctx: &mut MagicReformatCtx<'_>) {
    let missing_scalar_fields: Vec<InferredScalarField<'_>> = match inline.referencing_fields() {
        Some(_) => return,
        None => infer_missing_referencing_scalar_fields(inline),
    };

    // Filter out duplicate fields
    let missing_scalar_fields = missing_scalar_fields.iter().filter(|missing| {
        !inline
            .referencing_model()
            .scalar_fields()
            .any(|sf| sf.name() == missing.name)
    });

    for field in missing_scalar_fields {
        let field_name = &field.name;
        let field_type = if let Some(ft) = field.tpe.as_builtin_scalar() {
            ft.as_str()
        } else {
            return;
        };
        let arity = render_arity(field.arity);

        let mut attributes: String = String::new();
        if let Some((_datasource_name, _type_name, _args, span)) = field.blueprint.raw_native_type() {
            attributes.push('@');
            attributes.push_str(&ctx.original_schema[span.start..span.end]);
        }

        ctx.missing_bits.push(MissingBit {
            position: inline.referencing_model().ast_model().span.end - 1,
            content: format!("{field_name} {field_type}{arity} {attributes}\n"),
        });
    }
}

/// A scalar inferred by magic reformatting.
#[derive(Debug)]
struct InferredScalarField<'db> {
    name: String,
    arity: ast::FieldArity,
    tpe: parser_database::ScalarFieldType,
    blueprint: walkers::ScalarFieldWalker<'db>,
}

fn infer_missing_referencing_scalar_fields(inline: walkers::InlineRelationWalker<'_>) -> Vec<InferredScalarField<'_>> {
    match inline.referenced_model().unique_criterias().next() {
        Some(first_unique_criteria) => {
            first_unique_criteria
                .fields()
                .map(|field| {
                    let name = format!(
                        "{}{}",
                        camel_case(inline.referenced_model().name()),
                        pascal_case(field.name())
                    );

                    // we cannot have composite fields in a relation for now.
                    let field = field.as_scalar_field().unwrap();

                    if let Some(existing_field) =
                        inline.referencing_model().scalar_fields().find(|sf| sf.name() == name)
                    {
                        InferredScalarField {
                            name,
                            arity: existing_field.ast_field().arity,
                            tpe: existing_field.scalar_field_type(),
                            blueprint: field,
                        }
                    } else {
                        InferredScalarField {
                            name,
                            arity: inline
                                .forward_relation_field()
                                .map(|f| f.ast_field().arity)
                                .unwrap_or(ast::FieldArity::Optional),
                            tpe: field.scalar_field_type(),
                            blueprint: field,
                        }
                    }
                })
                .collect()
        }
        None => Vec::new(),
    }
}

fn pascal_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn camel_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
    }
}

/// The arity of the forward relation field. Works even without forward relation field.
fn forward_relation_field_arity(inline: walkers::InlineRelationWalker<'_>) -> ast::FieldArity {
    inline
        // First use the relation field itself if it exists.
        .forward_relation_field()
        .map(|rf| rf.ast_field().arity)
        // Otherwise, if we have fields that look right on the model, use these.
        .unwrap_or_else(|| {
            if infer_missing_referencing_scalar_fields(inline)
                .into_iter()
                .any(|f| f.arity.is_optional())
            {
                ast::FieldArity::Optional
            } else {
                ast::FieldArity::Required
            }
        })
}

fn render_arity(arity: ast::FieldArity) -> &'static str {
    match arity {
        ast::FieldArity::Required => "",
        ast::FieldArity::Optional => "?",
        ast::FieldArity::List => "[]",
    }
}

// the `fields: [...]` argument.
fn fields_argument(inline: walkers::InlineRelationWalker<'_>) -> String {
    let fields: Vec<InferredScalarField<'_>> = infer_missing_referencing_scalar_fields(inline);
    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    format!("fields: [{}]", field_names.join(", "))
}

// the `references: [...]` argument.
fn references_argument(inline: walkers::InlineRelationWalker<'_>) -> String {
    let field_names: Vec<&str> = inline.referenced_fields().map(|f| f.name()).collect();
    format!("references: [{}]", field_names.join(", "))
}

/// Assuming the last characters before span_end are newlines. We can fix this more thoroughly by
/// not including the newline in field spans.
fn before_newline(span_end: usize, original_schema: &str) -> usize {
    assert!(&original_schema[span_end - 1..span_end] == "\n");
    match &original_schema[span_end - 2..span_end - 1] {
        "\r" => span_end - 2,
        _ => span_end - 1,
    }
}
