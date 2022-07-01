use crate::{
    parser::{PrismaDatamodelParser, Rule},
    renderer::{get_sort_index_of_attribute, LineWriteable, Renderer, TableFormat},
};
use pest::Parser;

type Pair<'a> = pest::iterators::Pair<'a, Rule>;

/// Reformat the AST to a string, standardizing alignment and indentation.
pub fn reformat(input: &str, indent_width: usize) -> Option<String> {
    let mut ast = PrismaDatamodelParser::parse(Rule::schema, input).ok()?;
    let mut target_string = String::with_capacity(input.len());
    let mut renderer = Renderer::new(&mut target_string, indent_width);
    reformat_top(&mut renderer, ast.next().unwrap());

    // all schemas must end with a newline
    if !target_string.ends_with('\n') {
        target_string.push('\n');
    }

    Some(target_string)
}

fn reformat_top(target: &mut Renderer<'_>, pair: Pair<'_>) {
    let mut seen_at_least_one_top_level_element = false;

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::WHITESPACE => {}
            Rule::doc_comment | Rule::doc_comment_and_new_line => {}
            _ => {}
        };

        // new line handling outside of blocks:
        // * fold multiple new lines between blocks into one
        // * all new lines before the first block get removed
        if is_top_level_element(&current) {
            // separate top level elements with new lines
            if seen_at_least_one_top_level_element {
                target.write("\n");
            }
            seen_at_least_one_top_level_element = true;
        }

        match current.as_rule() {
            Rule::doc_comment | Rule::doc_comment_and_new_line => {
                comment(current.as_str(), target);
            }
            Rule::model_declaration => {
                let keyword = current
                    .clone()
                    .into_inner()
                    .find(|pair| matches!(pair.as_rule(), Rule::TYPE_KEYWORD | Rule::MODEL_KEYWORD))
                    .expect("Expected model or type keyword");

                match keyword.as_rule() {
                    Rule::TYPE_KEYWORD => reformat_composite_type(current, target),
                    Rule::MODEL_KEYWORD => reformat_model(current, target),
                    _ => unreachable!(),
                };
            }
            Rule::enum_declaration => reformat_enum(current, target),
            Rule::config_block => reformat_config_block(current, target),
            Rule::comment_block => {
                for comment_token in current.into_inner() {
                    comment(comment_token.as_str(), target);
                }
            }
            Rule::EOI => {}
            Rule::NEWLINE => {} // Do not render user provided newlines. We have a strong opinionation about new lines on the top level.
            _ => reformat_generic_pair(current, target),
        }
    }

    // FLUSH IT. Otherwise pending new lines do not get rendered.
    target.write("");
}

fn reformat_config_block(pair: Pair<'_>, target: &mut Renderer<'_>) {
    let keyword = pair
        .clone()
        .into_inner()
        .find(|p| [Rule::GENERATOR_KEYWORD, Rule::DATASOURCE_KEYWORD].contains(&p.as_rule()))
        .map(|tok| tok.as_str())
        .unwrap();

    reformat_block_element(
        keyword,
        target,
        pair,
        &(|table, _, pair| match pair.as_rule() {
            Rule::key_value => reformat_key_value(pair, table),
            _ => reformat_generic_pair(pair, table),
        }),
    );
}

fn reformat_key_value(pair: Pair<'_>, target: &mut TableFormat) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                target.write(current.as_str());
                target.write("=");
            }
            Rule::expression => {
                reformat_expression(current, &mut target.column_locked_writer_for(2));
            }
            Rule::doc_comment | Rule::doc_comment_and_new_line => {
                panic!("Comments inside config key/value not supported yet.")
            }
            _ => reformat_generic_pair(current, target),
        }
    }
}

fn reformat_model(pair: Pair<'_>, target: &mut Renderer<'_>) {
    reformat_block_element(
        "model",
        target,
        pair,
        &(|table, renderer, pair| {
            match pair.as_rule() {
                Rule::block_level_attribute => {
                    // model level attributes reset the table. -> .render() does that
                    table.render(renderer);
                    let attribute = pair.into_inner().next().unwrap();
                    reformat_attribute(attribute, "@@", renderer);
                }
                Rule::field_declaration => reformat_field(pair, table),
                _ => reformat_generic_pair(pair, table),
            }
        }),
    );
}

fn reformat_composite_type(pair: Pair<'_>, target: &mut Renderer<'_>) {
    reformat_block_element(
        "type",
        target,
        pair,
        &(|table, renderer, pair| {
            match pair.as_rule() {
                Rule::block_level_attribute => {
                    // model level attributes reset the table. -> .render() does that
                    table.render(renderer);
                    let attribute = pair.into_inner().next().unwrap();
                    reformat_attribute(attribute, "@@", renderer);
                }
                Rule::field_declaration => reformat_field(pair, table),
                _ => reformat_generic_pair(pair, table),
            }
        }),
    );
}

fn reformat_block_element(
    block_type: &str,
    renderer: &mut Renderer<'_>,
    pair: Pair<'_>,
    the_fn: &(dyn Fn(&mut TableFormat, &mut Renderer<'_>, Pair<'_>)),
) {
    let mut table = TableFormat::new();
    let mut block_has_opened = false;

    // sort attributes
    let mut attributes = extract_and_sort_attributes(pair.clone(), false);

    // used to add a new line between fields and block attributes if there isn't one already
    let mut last_line_was_empty = false;

    for current in pair.into_inner() {
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

                for pair in attributes.drain(..) {
                    the_fn(&mut table, renderer, pair);
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
                for current in current.into_inner() {
                    if block_has_opened {
                        comment(current.as_str(), &mut table.interleave_writer())
                    } else {
                        comment(current.as_str(), renderer)
                    }
                }
            }
            Rule::doc_comment | Rule::comment_and_new_line | Rule::doc_comment_and_new_line => {
                if block_has_opened {
                    comment(current.as_str(), &mut table.interleave_writer())
                } else {
                    comment(current.as_str(), renderer)
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
            _ => the_fn(&mut table, renderer, current),
        }
    }

    // End.
    table.render(renderer);
    renderer.indent_down();
    renderer.write("}");
    renderer.maybe_end_line();
}

fn reformat_enum(pair: Pair<'_>, target: &mut Renderer<'_>) {
    reformat_block_element(
        "enum",
        target,
        pair,
        &(|table, target, pair| {
            //
            match pair.as_rule() {
                Rule::block_level_attribute => {
                    let attribute = pair.into_inner().next().unwrap();
                    table.render(target);
                    reformat_attribute(attribute, "@@", target);
                    table.end_line();
                }
                Rule::enum_value_declaration => reformat_enum_entry(pair, table),
                _ => reformat_generic_pair(pair, table),
            }
        }),
    );
}

fn reformat_enum_entry(pair: Pair<'_>, target: &mut TableFormat) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::non_empty_identifier => target.write(current.as_str()),
            Rule::attribute => reformat_attribute(current, "@", &mut target.column_locked_writer_for(2)),
            Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
            _ => reformat_generic_pair(current, target),
        }
    }
}

fn extract_and_sort_attributes(pair: Pair<'_>, is_field_attribute: bool) -> Vec<Pair<'_>> {
    // get indices of attributes and store in separate Vector
    let mut attributes = Vec::new();
    for pair in pair.into_inner() {
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

fn reformat_field(pair: Pair<'_>, target: &mut TableFormat) {
    // extract and sort attributes
    let attributes = extract_and_sort_attributes(pair.clone(), true);

    // iterate through tokens and reorder attributes
    let mut attributes_count = 0;
    let inner_pairs_with_sorted_attributes = pair.into_inner().map(|p| match p.as_rule() {
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
                let mut writer = target.column_locked_writer_for(1);
                reformat_field_type(current, &mut writer);
            }
            // This is Prisma 1 thing, we should not render them. Example:
            //
            // ```no_run
            // model Site {
            //   name: String
            //   htmlTitle: String
            // }
            // ```
            Rule::LEGACY_COLON => {}
            Rule::attribute => reformat_attribute(current, "@", &mut target.column_locked_writer_for(2)),
            // This is a comment at the end of a field.
            Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
            // This is a comment before the field declaration. Hence it must be interlevaed.
            Rule::doc_comment_and_new_line => comment(current.as_str(), &mut target.interleave_writer()),
            Rule::NEWLINE => {} // we do the new lines ourselves
            _ => reformat_generic_pair(current, target),
        }
    }

    target.maybe_end_line();
}

fn reformat_field_type(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    assert!(pair.as_rule() == Rule::field_type);

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::optional_type => {
                target.write(get_identifier(current));
                target.write("?");
            }
            Rule::base_type | Rule::legacy_required_type => {
                target.write(get_identifier(current));
            }
            Rule::list_type | Rule::legacy_list_type => {
                target.write(get_identifier(current));
                target.write("[]");
            }
            Rule::unsupported_optional_list_type => {
                target.write(get_identifier(current));
                target.write("[]?");
            }
            _ => unreachable!(),
        }
    }
}

fn get_identifier(pair: Pair<'_>) -> &str {
    let ident_token = match pair.as_rule() {
        Rule::base_type => pair.as_str(),
        Rule::list_type
        | Rule::legacy_list_type
        | Rule::legacy_required_type
        | Rule::optional_type
        | Rule::unsupported_optional_list_type => {
            let ident_token = pair.into_inner().next().unwrap();
            assert!(ident_token.as_rule() == Rule::base_type);
            ident_token.as_str()
        }
        _ => unreachable!("Get identified failed. Unexpected input: {:#?}", pair),
    };

    ident_token
}

fn reformat_attribute(pair: Pair<'_>, owl: &str, target: &mut dyn LineWriteable) {
    let rule = pair.as_rule();
    assert!(rule == Rule::attribute);
    for current in pair.into_inner() {
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
            Rule::arguments_list => reformat_arguments_list(current, target),
            Rule::NEWLINE => (), // skip
            _ => reformat_generic_pair(current, target),
        }
    }
}

fn reformat_arguments_list(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    debug_assert_eq!(pair.as_rule(), Rule::arguments_list);

    target.write("(");

    for (idx, current) in pair.into_inner().enumerate() {
        let first_arg = idx == 0;
        match current.as_rule() {
            // This is a named arg.
            Rule::named_argument => {
                if !first_arg {
                    target.write(", ");
                }
                reformat_attribute_arg(current, target);
            }
            // This is a an unnamed arg.
            Rule::expression => {
                if !first_arg {
                    target.write(", ");
                }
                reformat_expression(current, target);
            }
            Rule::empty_argument => {
                if !first_arg {
                    target.write(", ");
                }
                reformat_attribute_arg(current, target);
            }
            Rule::trailing_comma => (), // skip it
            _ => reformat_generic_pair(current, target),
        };
    }

    target.write(")");
}

fn reformat_attribute_arg(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::argument_name => {
                target.write(current.as_str());
                target.write(": ");
            }
            Rule::expression => reformat_expression(current, target),
            Rule::doc_comment | Rule::doc_comment_and_new_line => {
                panic!("Comments inside attribute argument not supported yet.")
            }
            Rule::trailing_comma => (), // skip it
            _ => reformat_generic_pair(current, target),
        };
    }
}

fn reformat_expression(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::numeric_literal => target.write(current.as_str()),
            Rule::string_literal => target.write(current.as_str()),
            Rule::constant_literal => target.write(current.as_str()),
            Rule::function => reformat_function_expression(current, target),
            Rule::array_expression => reformat_array_expression(current, target),
            Rule::doc_comment | Rule::doc_comment_and_new_line => {
                panic!("Comments inside expressions not supported yet.")
            }
            _ => reformat_generic_pair(current, target),
        }
    }
}

fn reformat_array_expression(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    target.write("[");
    let mut expr_count = 0;

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::expression => {
                if expr_count > 0 {
                    target.write(", ");
                }
                reformat_expression(current, target);
                expr_count += 1;
            }
            Rule::doc_comment | Rule::doc_comment_and_new_line => {
                panic!("Comments inside expressions not supported yet.")
            }
            _ => reformat_generic_pair(current, target),
        }
    }

    target.write("]");
}

fn reformat_function_expression(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::function_name => {
                target.write(current.as_str());
            }
            Rule::arguments_list => reformat_arguments_list(current, target),
            _ => reformat_generic_pair(current, target),
        }
    }
}

fn reformat_generic_pair(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    match pair.as_rule() {
        Rule::NEWLINE => target.end_line(),
        Rule::comment_block => {
            for pair in pair.into_inner() {
                reformat_generic_pair(pair, target)
            }
        }
        Rule::doc_comment_and_new_line | Rule::doc_comment | Rule::comment_and_new_line | Rule::comment => {
            comment(pair.as_str(), target)
        }
        Rule::WHITESPACE => {} // we are very opinionated about whitespace and hence ignore user input
        Rule::CATCH_ALL | Rule::BLOCK_LEVEL_CATCH_ALL => {
            target.write(pair.as_str());
        }
        _ => unreachable!(
            "Encountered impossible declaration during formatting: {:?}",
            pair.clone().tokens()
        ),
    }
}

fn comment(comment_text: &str, target: &mut dyn LineWriteable) {
    let trimmed = strip_new_line(comment_text);
    let trimmed = trimmed.trim();

    target.write(trimmed);
    target.end_line();
}

fn strip_new_line(str: &str) -> &str {
    if str.ends_with('\n') {
        &str[0..str.len() - 1] // slice away line break.
    } else {
        str
    }
}

fn is_top_level_element(pair: &Pair<'_>) -> bool {
    matches!(
        pair.as_rule(),
        Rule::model_declaration | Rule::enum_declaration | Rule::config_block | Rule::type_alias | Rule::comment_block
    )
}
