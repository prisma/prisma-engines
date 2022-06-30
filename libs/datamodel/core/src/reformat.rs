mod helpers;

use crate::ParserDatabase;
use helpers::*;
use parser_database::walkers;
use pest::{iterators::Pair, Parser};
use schema_ast::{
    ast,
    parser::{PrismaDatamodelParser, Rule},
    renderer::*,
};
use std::borrow::Cow;

/// Returns either the reformatted schema, or the original input if we can't reformat. This happens
/// if and only if the source does not parse to a well formed AST.
pub fn reformat(source: &str, indent_width: usize) -> Option<String> {
    let db = crate::parse_schema_ast(source).ok().and_then(|ast| {
        let mut diagnostics = diagnostics::Diagnostics::new();
        let db = parser_database::ParserDatabase::new(ast, &mut diagnostics);
        diagnostics.to_result().ok().map(move |_| db)
    });
    let source_to_reformat: Cow<'_, str> = match db {
        Some(db) => {
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
        }
        None => Cow::Borrowed(source),
    };

    Reformatter::new(&source_to_reformat).reformat_internal(indent_width)
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

struct Reformatter<'a> {
    input: &'a str,
}

impl<'a> Reformatter<'a> {
    fn new(input: &'a str) -> Self {
        Reformatter { input }
    }

    fn reformat_internal(self, indent_width: usize) -> Option<String> {
        let mut ast = PrismaDatamodelParser::parse(Rule::schema, self.input).ok()?;
        let mut target_string = String::with_capacity(self.input.len());
        let mut renderer = Renderer::new(&mut target_string, indent_width);
        self.reformat_top(&mut renderer, &ast.next().unwrap());

        // all schemas must end with a newline
        if !target_string.ends_with('\n') {
            target_string.push('\n');
        }

        Some(target_string)
    }

    fn reformat_top(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        let mut seen_at_least_one_top_level_element = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {}
                _ => {}
            };

            // new line handling outside of blocks:
            // * fold multiple new lines between blocks into one
            // * all new lines before the first block get removed
            if current.is_top_level_element() {
                // separate top level elements with new lines
                if seen_at_least_one_top_level_element {
                    target.write("\n");
                }
                seen_at_least_one_top_level_element = true;
            }

            match current.as_rule() {
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(target, current.as_str());
                }
                Rule::model_declaration => {
                    let keyword = current
                        .clone()
                        .into_inner()
                        .find(|pair| matches!(pair.as_rule(), Rule::TYPE_KEYWORD | Rule::MODEL_KEYWORD))
                        .expect("Expected model or type keyword");

                    match keyword.as_rule() {
                        Rule::TYPE_KEYWORD => self.reformat_composite_type(target, &current),
                        Rule::MODEL_KEYWORD => self.reformat_model(target, &current),
                        _ => unreachable!(),
                    };
                }
                Rule::enum_declaration => self.reformat_enum(target, &current),
                Rule::config_block => self.reformat_config_block(target, &current),
                Rule::comment_block => {
                    for comment_token in current.clone().into_inner() {
                        comment(target, comment_token.as_str());
                    }
                }
                Rule::EOI => {}
                Rule::NEWLINE => {} // Do not render user provided newlines. We have a strong opinionation about new lines on the top level.
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        // FLUSH IT. Otherwise pending new lines do not get rendered.
        target.write("");
    }

    fn reformat_config_block(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        let keyword = token
            .clone()
            .into_inner()
            .find(|p| [Rule::GENERATOR_KEYWORD, Rule::DATASOURCE_KEYWORD].contains(&p.as_rule()))
            .map(|tok| tok.as_str())
            .unwrap();

        self.reformat_block_element(
            keyword,
            target,
            token,
            &(|table, _, token| match token.as_rule() {
                Rule::key_value => Self::reformat_key_value(table, token),
                _ => Self::reformat_generic_token(table, token),
            }),
        );
    }

    fn reformat_key_value(target: &mut TableFormat, token: &Token<'_>) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    target.write(current.as_str());
                    target.write("=");
                }
                Rule::expression => {
                    Self::reformat_expression(&mut target.column_locked_writer_for(2), &current);
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside config key/value not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn reformat_model(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element(
            "model",
            target,
            token,
            &(|table, renderer, token| {
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        // model level attributes reset the table. -> .render() does that
                        table.render(renderer);
                        Self::reformat_attribute(renderer, token, "@@");
                    }
                    Rule::field_declaration => self.reformat_field(table, token),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
        );
    }

    fn reformat_composite_type(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element(
            "type",
            target,
            token,
            &(|table, renderer, token| {
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        // model level attributes reset the table. -> .render() does that
                        table.render(renderer);
                        Self::reformat_attribute(renderer, token, "@@");
                    }
                    Rule::field_declaration => self.reformat_field(table, token),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
        );
    }

    fn reformat_block_element(
        &self,
        block_type: &'a str,
        renderer: &'a mut Renderer<'_>,
        token: &'a Token<'_>,
        the_fn: &(dyn Fn(&mut TableFormat, &mut Renderer<'_>, &Token<'_>) + 'a),
    ) {
        let mut table = TableFormat::new();
        let mut block_has_opened = false;

        // sort attributes
        let attributes = Self::extract_and_sort_attributes(token, false);

        // used to add a new line between fields and block attributes if there isn't one already
        let mut last_line_was_empty = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::MODEL_KEYWORD | Rule::TYPE_KEYWORD | Rule::GENERATOR_KEYWORD | Rule::DATASOURCE_KEYWORD => (),
                Rule::BLOCK_OPEN => {
                    block_has_opened = true;
                }
                Rule::BLOCK_CLOSE => {
                    // New line between fields and attributes
                    // only if there isn't already a new line in between
                    if !attributes.is_empty() && !last_line_was_empty {
                        table.render(renderer);
                        table = TableFormat::new();
                        renderer.end_line();
                    }

                    for d in &attributes {
                        the_fn(&mut table, renderer, d);
                        // New line after each block attribute
                        table.render(renderer);
                        table = TableFormat::new();
                        renderer.maybe_end_line();
                    }
                }

                Rule::block_level_attribute => {}

                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    // Begin.
                    let block_name = current.as_str();
                    renderer.write(&format!("{} {} {{", block_type, block_name));
                    renderer.end_line();
                    renderer.indent_up();
                }
                Rule::comment_block => {
                    for current in current.clone().into_inner() {
                        if block_has_opened {
                            comment(&mut table.interleave_writer(), current.as_str())
                        } else {
                            comment(renderer, current.as_str())
                        }
                    }
                }
                Rule::doc_comment | Rule::comment_and_new_line | Rule::doc_comment_and_new_line => {
                    if block_has_opened {
                        comment(&mut table.interleave_writer(), current.as_str())
                    } else {
                        comment(renderer, current.as_str())
                    }
                }
                Rule::NEWLINE => {
                    if block_has_opened {
                        last_line_was_empty = renderer.line_empty();

                        // do not render newlines before the block
                        // Reset the table layout on a newline.
                        table.render(renderer);
                        table = TableFormat::new();
                        renderer.end_line();
                    }
                }
                Rule::BLOCK_LEVEL_CATCH_ALL => {
                    table.interleave(strip_new_line(current.as_str()));
                }
                _ => the_fn(&mut table, renderer, &current),
            }
        }

        // End.
        table.render(renderer);
        renderer.indent_down();
        renderer.write("}");
        renderer.maybe_end_line();
    }

    fn reformat_enum(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element(
            "enum",
            target,
            token,
            &(|table, target, token| {
                //
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        table.render(target);
                        Self::reformat_attribute(target, token, "@@");
                        table.end_line();
                    }
                    Rule::enum_value_declaration => Self::reformat_enum_entry(table, token),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
        );
    }

    fn reformat_enum_entry(target: &mut TableFormat, token: &Token<'_>) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier => target.write(current.as_str()),
                Rule::attribute => Self::reformat_attribute(&mut target.column_locked_writer_for(2), &current, "@"),
                Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn extract_and_sort_attributes<'i>(token: &'i Token<'_>, is_field_attribute: bool) -> Vec<Pair<'i, Rule>> {
        // get indices of attributes and store in separate Vector
        let mut attributes = Vec::new();
        for pair in token.clone().into_inner() {
            if is_field_attribute {
                if let Rule::attribute = pair.as_rule() {
                    attributes.push(pair)
                }
            } else if let Rule::block_level_attribute = pair.as_rule() {
                attributes.push(pair)
            }
        }

        // sort attributes
        attributes.sort_by(|a, b| {
            let sort_index_a = get_sort_index_of_attribute(is_field_attribute, a.as_str());
            let sort_index_b = get_sort_index_of_attribute(is_field_attribute, b.as_str());
            sort_index_a.cmp(&sort_index_b)
        });
        attributes
    }

    fn reformat_field(&self, target: &mut TableFormat, token: &Token<'_>) {
        // extract and sort attributes
        let attributes = Self::extract_and_sort_attributes(token, true);

        // iterate through tokens and reorder attributes
        let mut attributes_count = 0;
        let inner_pairs_with_sorted_attributes = token.clone().into_inner().map(|p| match p.as_rule() {
            Rule::attribute => {
                attributes_count += 1;
                attributes[attributes_count - 1].clone()
            }
            _ => p,
        });

        // Write existing attributes first.
        for current in inner_pairs_with_sorted_attributes {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    target.write(current.as_str());
                }
                Rule::field_type => {
                    target.write(&Self::reformat_field_type(&current));
                }
                Rule::attribute => Self::reformat_attribute(&mut target.column_locked_writer_for(2), &current, "@"),
                // This is a comment at the end of a field.
                Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
                // This is a comment before the field declaration. Hence it must be interlevaed.
                Rule::doc_comment_and_new_line => comment(&mut target.interleave_writer(), current.as_str()),
                Rule::NEWLINE => {} // we do the new lines ourselves
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        target.maybe_end_line();
    }

    fn reformat_field_type(token: &Token<'_>) -> String {
        assert!(token.as_rule() == Rule::field_type);

        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::optional_type => {
                    builder.write(Self::get_identifier(current));
                    builder.write("?");
                }
                Rule::base_type | Rule::legacy_required_type => {
                    builder.write(Self::get_identifier(current));
                }
                Rule::list_type | Rule::legacy_list_type => {
                    builder.write(Self::get_identifier(current));
                    builder.write("[]");
                }
                Rule::unsupported_optional_list_type => {
                    builder.write(Self::get_identifier(current));
                    builder.write("[]?");
                }
                _ => unreachable!(),
            }
        }

        builder.to_string()
    }

    fn get_identifier(token: Token<'_>) -> &str {
        let ident_token = match token.as_rule() {
            Rule::base_type => token.as_str(),
            Rule::list_type
            | Rule::legacy_list_type
            | Rule::legacy_required_type
            | Rule::optional_type
            | Rule::unsupported_optional_list_type => {
                let ident_token = token.into_inner().next().unwrap();
                assert!(ident_token.as_rule() == Rule::base_type);
                ident_token.as_str()
            }
            _ => unreachable!("Get identified failed. Unexpected input: {:#?}", token),
        };

        ident_token
    }

    fn reformat_attribute(target: &mut dyn LineWriteable, token: &Token<'_>, owl: &str) {
        let token = Self::unpack_token_to_find_matching_rule(token.clone(), Rule::attribute);
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::attribute_name => {
                    if !target.line_empty() {
                        target.write(" ");
                    }
                    target.write(owl);
                    target.write(current.as_str());
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                Rule::arguments_list => Self::reformat_arguments_list(target, &current),
                Rule::NEWLINE => (), // skip
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn unpack_token_to_find_matching_rule(token: Token<'_>, rule: Rule) -> Token<'_> {
        if token.as_rule() == rule {
            token
        } else {
            let error_msg = format!("Token matching rule {:?} not found in: {:?}", &rule, &token.as_str());
            for token in token.into_inner() {
                if token.as_rule() == rule {
                    return token;
                }
            }
            panic!("{}", error_msg)
        }
    }

    fn reformat_arguments_list(target: &mut dyn LineWriteable, token: &Token<'_>) {
        debug_assert_eq!(token.as_rule(), Rule::arguments_list);

        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            match current.as_rule() {
                // This is a named arg.
                Rule::named_argument => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_attribute_arg(&mut builder, &current);
                }
                // This is a an unnamed arg.
                Rule::expression => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_expression(&mut builder, &current);
                }
                Rule::empty_argument => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_attribute_arg(&mut builder, &current);
                }
                Rule::trailing_comma => (), // skip it
                _ => Self::reformat_generic_token(target, &current),
            };
        }

        target.write("(");
        target.write(&builder.to_string());
        target.write(")");
    }

    fn reformat_attribute_arg(target: &mut dyn LineWriteable, token: &Token<'_>) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::argument_name => {
                    target.write(current.as_str());
                    target.write(": ");
                }
                Rule::expression => Self::reformat_expression(target, &current),
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attribute argument not supported yet.")
                }
                Rule::trailing_comma => (), // skip it
                _ => Self::reformat_generic_token(target, &current),
            };
        }
    }

    /// Parses an expression, given a Pest parser token.
    fn reformat_expression(target: &mut dyn LineWriteable, token: &Token<'_>) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::numeric_literal => target.write(current.as_str()),
                Rule::string_literal => target.write(current.as_str()),
                Rule::constant_literal => target.write(current.as_str()),
                Rule::function => Self::reformat_function_expression(target, &current),
                Rule::array_expression => Self::reformat_array_expression(target, &current),
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn reformat_array_expression(target: &mut dyn LineWriteable, token: &Token<'_>) {
        target.write("[");
        let mut expr_count = 0;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::expression => {
                    if expr_count > 0 {
                        target.write(", ");
                    }
                    Self::reformat_expression(target, &current);
                    expr_count += 1;
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        target.write("]");
    }

    fn reformat_function_expression(target: &mut dyn LineWriteable, token: &Token<'_>) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::function_name => {
                    target.write(current.as_str());
                }
                Rule::arguments_list => Self::reformat_arguments_list(target, &current),
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn reformat_generic_token(target: &mut dyn LineWriteable, token: &Token<'_>) {
        match token.as_rule() {
            Rule::NEWLINE => target.end_line(),
            Rule::comment_block => {
                for token in token.clone().into_inner() {
                    Self::reformat_generic_token(target, &token)
                }
            }
            Rule::doc_comment_and_new_line | Rule::doc_comment | Rule::comment_and_new_line | Rule::comment => {
                comment(target, token.as_str())
            }
            Rule::WHITESPACE => {} // we are very opinionated about whitespace and hence ignore user input
            // This is Prisma 1 thing, we should not render them. Example:
            //
            // ```no_run
            // model Site {
            //   name: String
            //   htmlTitle: String
            // }
            // ```
            Rule::LEGACY_COLON => {}
            Rule::CATCH_ALL | Rule::BLOCK_LEVEL_CATCH_ALL => {
                target.write(token.as_str());
            }
            _ => unreachable!(
                "Encountered impossible declaration during formatting: {:?}",
                token.clone().tokens()
            ),
        }
    }
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
