use crate::{
    parser::{PrismaDatamodelParser, Rule},
    renderer::{LineWriteable, Renderer, TableFormat},
};
use pest::Parser;
use std::iter::Peekable;

type Pair<'a> = pest::iterators::Pair<'a, Rule>;

/// Reformat a PSL string.
pub fn reformat(input: &str, indent_width: usize) -> Option<String> {
    let mut ast = PrismaDatamodelParser::parse(Rule::schema, input).ok()?;
    let mut renderer = Renderer::new(indent_width);
    renderer.stream.reserve(input.len() / 2);
    reformat_top(&mut renderer, ast.next().unwrap());

    // all schemas must end with a newline
    if !renderer.stream.ends_with('\n') {
        renderer.stream.push('\n');
    }

    Some(renderer.stream)
}

fn reformat_top(target: &mut Renderer, pair: Pair<'_>) {
    let mut pairs = pair.into_inner().peekable();
    eat_empty_lines(&mut pairs);

    while let Some(current) = pairs.next() {
        match current.as_rule() {
            Rule::model_declaration | Rule::enum_declaration | Rule::config_block => {
                reformat_block_element(current, target)
            }
            Rule::comment_block => {
                let mut table = Default::default();
                reformat_comment_block(current, &mut table);
                table.render(target);
            }
            Rule::empty_lines => {
                match pairs.peek().map(|b| b.as_rule()) {
                    None | Some(Rule::EOI) => (), // skip the last empty lines
                    _ => target.end_line(),
                }
            }
            Rule::CATCH_ALL | Rule::BLOCK_LEVEL_CATCH_ALL | Rule::arbitrary_block | Rule::type_alias => {
                target.write(current.as_str());
            }
            Rule::EOI => {}
            _ => unreachable(&current),
        }
    }
}

fn reformat_key_value(pair: Pair<'_>, table: &mut TableFormat) {
    table.start_new_line();
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => table.column_locked_writer_for(0).write(current.as_str()),
            Rule::expression => {
                let mut writer = table.column_locked_writer_for(1);
                writer.write("= ");
                reformat_expression(current, &mut writer);
            }
            Rule::trailing_comment => table.append_suffix_to_current_row(current.as_str()),
            _ => unreachable(&current),
        }
    }
}

fn reformat_block_element(pair: Pair<'_>, renderer: &mut Renderer) {
    let mut pairs = pair.into_inner().peekable();
    let block_type = pairs.next().unwrap().as_str();

    loop {
        let current = match pairs.next() {
            Some(current) => current,
            None => return,
        };

        match current.as_rule() {
            Rule::BLOCK_OPEN => {
                // Reformat away the empty lines at the beginning of the block.
                eat_empty_lines(&mut pairs);
            }
            Rule::BLOCK_CLOSE => {}

            Rule::model_contents | Rule::config_contents | Rule::enum_contents => {
                reformat_block_contents(&mut current.into_inner().peekable(), renderer)
            }

            Rule::identifier => {
                let block_name = current.as_str();
                renderer.write(block_type);
                renderer.write(" ");
                renderer.write(block_name);
                renderer.write(" {");
                renderer.end_line();
                renderer.indent_up();
            }

            _ => unreachable(&current),
        }
    }
}

fn reformat_block_contents<'a>(
    pairs: &mut Peekable<impl Iterator<Item = pest::iterators::Pair<'a, Rule>>>,
    renderer: &mut Renderer,
) {
    let mut attributes: Vec<(Option<Pair<'_>>, Pair<'_>)> = Vec::new(); // (Option<doc_comment>, attribute)
    let mut table = TableFormat::default();

    let mut pending_block_comment = None; // comment before an attribute

    // Reformat away the empty lines at the beginning of the block.
    eat_empty_lines(pairs);

    loop {
        let ate_empty_lines = eat_empty_lines(pairs);

        // Decide what to do with the empty lines.
        if ate_empty_lines {
            match pairs.peek().map(|pair| pair.as_rule()) {
                None | Some(Rule::block_attribute) | Some(Rule::comment_block) => {
                    // Reformat away the empty lines at the end of blocks and before attributes (we
                    // re-add them later).
                }
                Some(_) => {
                    // Reset the table layout on an empty line.
                    table.render(renderer);
                    table = TableFormat::default();
                    table.start_new_line();
                }
            }
        }
        let current = match pairs.next() {
            Some(current) => current,
            None => {
                // Flush current table.
                table.render(renderer);
                table = Default::default();

                // We are going to render the block attributes: new line.
                if !attributes.is_empty() {
                    table.start_new_line();
                }

                sort_attributes(&mut attributes[..]);

                for (comment, pair) in attributes.drain(..) {
                    if let Some(comment) = comment {
                        reformat_comment_block(comment, &mut table);
                    }
                    reformat_block_attribute(pair, &mut table);
                }

                table.render(renderer);
                renderer.indent_down();
                renderer.write("}");
                renderer.end_line();
                return;
            }
        };

        match current.as_rule() {
            Rule::comment_block => {
                if pairs.peek().map(|pair| pair.as_rule()) == Some(Rule::block_attribute) {
                    pending_block_comment = Some(current.clone()); // move it with the attribute
                } else {
                    if ate_empty_lines {
                        table.render(renderer);
                        table = Default::default();
                        table.start_new_line();
                    }
                    reformat_comment_block(current, &mut table);
                }
            }

            Rule::field_declaration => reformat_field(current, &mut table),
            Rule::key_value => reformat_key_value(current, &mut table),
            Rule::enum_value_declaration => reformat_enum_entry(current, &mut table),
            Rule::block_attribute => attributes.push((pending_block_comment.take(), current)),
            Rule::CATCH_ALL | Rule::BLOCK_LEVEL_CATCH_ALL => {
                table.interleave(current.as_str().trim_end_matches('\n'));
            }
            _ => unreachable(&current),
        }
    }
}

fn reformat_block_attribute(pair: Pair<'_>, table: &mut TableFormat) {
    debug_assert!(pair.as_rule() == Rule::block_attribute);
    table.start_new_line();
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::path => {
                let mut writer = table.column_locked_writer_for(0);
                writer.write("@@");
                writer.write(current.as_str());
            }
            Rule::arguments_list => reformat_arguments_list(current, &mut table.column_locked_writer_for(0)),
            Rule::trailing_comment => table.append_suffix_to_current_row(current.as_str()),
            _ => unreachable(&current),
        }
    }
}

fn reformat_enum_entry(pair: Pair<'_>, table: &mut TableFormat) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => {
                table.start_new_line();
                table.column_locked_writer_for(0).write(current.as_str())
            }
            Rule::field_attribute => {
                let mut writer = table.column_locked_writer_for(1);
                writer.write("@");
                reformat_function_call(current, &mut writer)
            }
            Rule::trailing_comment => table.append_suffix_to_current_row(current.as_str()),
            Rule::comment_block => reformat_comment_block(current, table),
            _ => unreachable(&current),
        }
    }
}

fn sort_attributes(attributes: &mut [(Option<Pair<'_>>, Pair<'_>)]) {
    attributes.sort_by(|(_, a), (_, b)| {
        let sort_index_a = get_sort_index_of_attribute(a.clone());
        let sort_index_b = get_sort_index_of_attribute(b.clone());
        sort_index_a.cmp(&sort_index_b)
    });
}

fn reformat_field(pair: Pair<'_>, table: &mut TableFormat) {
    let mut attributes = Vec::new();

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => {
                table.start_new_line();
                table
                    .column_locked_writer_for(FIELD_NAME_COLUMN)
                    .write(current.as_str());
            }
            Rule::field_type => {
                let mut writer = table.column_locked_writer_for(FIELD_TYPE_COLUMN);
                reformat_field_type(current, &mut writer);
            }
            // This is a Prisma 1 thing, we should not render them. Example:
            //
            // ```no_run
            // model Site {
            //   name: String
            //   htmlTitle: String
            // }
            // ```
            Rule::LEGACY_COLON => {}
            Rule::trailing_comment => table.append_suffix_to_current_row(current.as_str()),
            Rule::field_attribute => {
                attributes.push((None, current));
            }
            _ => unreachable(&current),
        }
    }

    let mut attributes_writer = table.column_locked_writer_for(FIELD_ATTRIBUTES_COLUMN);
    sort_attributes(&mut attributes[..]);
    let mut attributes = attributes.into_iter().peekable();
    while let Some((_, attribute)) = attributes.next() {
        attributes_writer.write("@");
        reformat_function_call(attribute, &mut attributes_writer);
        if attributes.peek().is_some() {
            attributes_writer.write(" ");
        }
    }
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
            _ => unreachable(&current),
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
        _ => unreachable(&pair),
    };

    ident_token
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
            _ => unreachable(&current),
        };
    }

    target.write(")");
}

fn reformat_attribute_arg(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::identifier => {
                target.write(current.as_str());
                target.write(": ");
            }
            Rule::expression => reformat_expression(current, target),
            Rule::trailing_comma => (), // skip it
            _ => unreachable(&current),
        };
    }
}

fn reformat_expression(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::numeric_literal => target.write(current.as_str()),
            Rule::string_literal => target.write(current.as_str()),
            Rule::path => target.write(current.as_str()),
            Rule::function_call => reformat_function_call(current, target),
            Rule::array_expression => reformat_array_expression(current, target),
            _ => unreachable(&current),
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
            _ => unreachable(&current),
        }
    }

    target.write("]");
}

fn reformat_function_call(pair: Pair<'_>, target: &mut dyn LineWriteable) {
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::path => target.write(current.as_str()),
            Rule::arguments_list => reformat_arguments_list(current, target),
            _ => unreachable(&current),
        }
    }
}

#[track_caller]
fn unreachable(pair: &Pair<'_>) -> ! {
    unreachable!("Encountered impossible declaration during formatting: {pair:?}")
}

fn reformat_comment_block(pair: Pair<'_>, table: &mut TableFormat) {
    assert!(pair.as_rule() == Rule::comment_block);
    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::comment | Rule::doc_comment => {
                table.start_new_line();
                let prefix = if current.as_rule() == Rule::doc_comment {
                    "///"
                } else {
                    "//"
                };

                table.append_suffix_to_current_row(prefix);
                for inner in current.into_inner() {
                    match inner.as_rule() {
                        Rule::doc_content => table.append_suffix_to_current_row(inner.as_str()),
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Returns `true` if at least one empty line was eaten.
fn eat_empty_lines<'a>(pairs: &mut Peekable<impl Iterator<Item = Pair<'a>>>) -> bool {
    match pairs.peek().map(|p| p.as_rule()) {
        Some(Rule::empty_lines) => {
            pairs.next(); // eat it
            true
        }
        _ => false,
    }
}

const FIELD_NAME_COLUMN: usize = 0;
const FIELD_TYPE_COLUMN: usize = 1;
const FIELD_ATTRIBUTES_COLUMN: usize = 2;

fn get_sort_index_of_attribute(attribute: Pair<'_>) -> usize {
    let path = attribute.into_inner().next().unwrap();
    debug_assert_eq!(path.as_rule(), Rule::path);
    let path = path.as_str();
    let correct_order: &[&str] = &[
        "id",
        "unique",
        "default",
        "updatedAt",
        "index",
        "fulltext",
        "map",
        "relation",
        "ignore",
    ];

    let pos = correct_order.iter().position(|p| path == *p);
    pos.unwrap_or(usize::MAX)
}
