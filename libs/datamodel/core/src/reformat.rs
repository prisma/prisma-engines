mod helpers;

use crate::{Diagnostics, ParserDatabase};
use datamodel_connector::Span;
use helpers::*;
use parser_database::walkers;
use pest::{iterators::Pair, Parser};
use schema_ast::{
    ast,
    parser::{PrismaDatamodelParser, Rule},
    renderer::*,
};

/// Returns either the reformatted schema, or the original input if we can't reformat. This happens
/// if and only if the source does not parse to a valid AST.
pub fn reformat(source: &str, indent_width: usize) -> Result<String, &str> {
    Reformatter::new(source).reformat_internal(indent_width)
}

fn parse_datamodel_for_formatter(input: &str) -> Result<ParserDatabase, Diagnostics> {
    let ast = crate::parse_schema_ast(input)?;
    let mut diagnostics = diagnostics::Diagnostics::new();
    let db = parser_database::ParserDatabase::new(ast, &mut diagnostics);
    diagnostics.to_result()?;
    Ok(db)
}

struct Reformatter<'a> {
    input: &'a str,
    missing_fields: Vec<MissingField>,
    missing_field_attributes: Vec<MissingFieldAttribute>,
    missing_relation_attribute_args: Vec<MissingRelationAttributeArg>,
}

impl<'a> Reformatter<'a> {
    fn new(input: &'a str) -> Self {
        match parse_datamodel_for_formatter(input) {
            Ok(db) => {
                let missing_fields = find_all_missing_fields(&db);
                let missing_field_attributes = find_all_missing_attributes(&db);
                let missing_relation_attribute_args = find_all_missing_relation_attribute_args(&db);

                Reformatter {
                    input,
                    missing_fields,
                    missing_field_attributes,
                    missing_relation_attribute_args,
                }
            }
            _ => Reformatter {
                input,
                missing_field_attributes: Vec::new(),
                missing_relation_attribute_args: Vec::new(),
                missing_fields: Vec::new(),
            },
        }
    }

    fn reformat_internal(self, indent_width: usize) -> Result<String, &'a str> {
        let mut ast = match PrismaDatamodelParser::parse(Rule::schema, self.input) {
            Ok(ast) => ast,
            Err(_) => return Err(self.input),
        };
        let mut target_string = String::with_capacity(self.input.len());
        let mut renderer = Renderer::new(&mut target_string, indent_width);
        self.reformat_top(&mut renderer, &ast.next().unwrap());

        // all schemas must end with a newline
        if !target_string.ends_with('\n') {
            target_string.push('\n');
        }

        Ok(target_string)
    }

    fn reformat_top(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        let mut types_table = TableFormat::new();
        let mut types_mode = false;
        let mut seen_at_least_one_top_level_element = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {}
                Rule::type_alias => {
                    types_mode = true;
                }
                _ => {
                    if types_mode {
                        types_mode = false;
                        // For all other ones, reset types_table.
                        types_table.render(target);
                        types_table = TableFormat::new();
                        target.maybe_end_line();
                    }
                }
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
                    if types_mode {
                        comment(&mut types_table.interleave_writer(), current.as_str());
                    } else {
                        comment(target, current.as_str());
                    }
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
                Rule::type_alias => {
                    if !types_mode {
                        panic!("Renderer not in type mode.");
                    }
                    Self::reformat_type_alias(&mut types_table, &current);
                }
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
            &(|table, _, token, _| match token.as_rule() {
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
        self.reformat_block_element_internal(
            "model",
            target,
            token,
            &(|table, renderer, token, model_name| {
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        // model level attributes reset the table. -> .render() does that
                        table.render(renderer);
                        Self::reformat_attribute(renderer, token, "@@", vec![]);
                    }
                    Rule::field_declaration => self.reformat_field(table, token, model_name),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
            &(|table, _, model_name| {
                for missing_back_relation_field in &self.missing_fields {
                    if missing_back_relation_field.model.as_str() == model_name {
                        Renderer::render_field(table, &missing_back_relation_field.field, false);
                    }
                }
            }),
        );
    }

    fn reformat_composite_type(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element_internal(
            "type",
            target,
            token,
            &(|table, renderer, token, model_name| {
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        // model level attributes reset the table. -> .render() does that
                        table.render(renderer);
                        Self::reformat_attribute(renderer, token, "@@", vec![]);
                    }
                    Rule::field_declaration => self.reformat_field(table, token, model_name),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
            &(|_, _, _| ()),
        );
    }

    fn reformat_block_element(
        &self,
        block_type: &'a str,
        renderer: &'a mut Renderer<'_>,
        token: &'a Token<'_>,
        the_fn: &(dyn Fn(&mut TableFormat, &mut Renderer<'_>, &Token<'_>, &str) + 'a),
    ) {
        self.reformat_block_element_internal(block_type, renderer, token, the_fn, {
            // a no op
            &(|_, _, _| ())
        })
    }

    fn reformat_block_element_internal(
        &self,
        block_type: &'a str,
        renderer: &'a mut Renderer<'_>,
        token: &'a Token<'_>,
        the_fn: &(dyn Fn(&mut TableFormat, &mut Renderer<'_>, &Token<'_>, &str) + 'a),
        after_fn: &(dyn Fn(&mut TableFormat, &mut Renderer<'_>, &str) + 'a),
    ) {
        let mut table = TableFormat::new();
        let mut block_name = "";
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
                        the_fn(&mut table, renderer, d, block_name);
                        // New line after each block attribute
                        table.render(renderer);
                        table = TableFormat::new();
                        renderer.maybe_end_line();
                    }
                }

                Rule::block_level_attribute => {}

                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    // Begin.
                    block_name = current.as_str();
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
                _ => the_fn(&mut table, renderer, &current, block_name),
            }
        }

        after_fn(&mut table, renderer, block_name);

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
            &(|table, target, token, _| {
                //
                match token.as_rule() {
                    Rule::block_level_attribute => {
                        table.render(target);
                        Self::reformat_attribute(target, token, "@@", vec![]);
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
                Rule::attribute => {
                    Self::reformat_attribute(&mut target.column_locked_writer_for(2), &current, "@", vec![])
                }
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

    fn reformat_field(&self, target: &mut TableFormat, token: &Token<'_>, model_name: &str) {
        let field_name = token
            .clone()
            .into_inner()
            .find(|tok| tok.as_rule() == Rule::non_empty_identifier)
            .unwrap()
            .as_str();

        // extract and sort attributes
        let attributes = Self::extract_and_sort_attributes(token, true);

        // iterate through tokens and reorder attributes
        let mut count = 0;
        let inner_pairs_with_sorted_attributes = token.clone().into_inner().map(|p| match p.as_rule() {
            Rule::attribute => {
                count += 1;
                attributes[count - 1].clone()
            }
            _ => p,
        });

        // write to target
        for current in inner_pairs_with_sorted_attributes {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    target.write(current.as_str());
                }
                Rule::field_type => {
                    target.write(&Self::reformat_field_type(&current));
                }
                Rule::attribute => {
                    let missing_relation_args: Vec<&MissingRelationAttributeArg> = self
                        .missing_relation_attribute_args
                        .iter()
                        .filter(|arg| arg.model == model_name && arg.field == *field_name)
                        .collect();

                    Self::reformat_attribute(
                        &mut target.column_locked_writer_for(2),
                        &current,
                        "@",
                        missing_relation_args,
                    )
                }
                // This is a comment at the end of a field.
                Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
                // This is a comment before the field declaration. Hence it must be interlevaed.
                Rule::doc_comment_and_new_line => comment(&mut target.interleave_writer(), current.as_str()),
                Rule::NEWLINE => {} // we do the new lines ourselves
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        for missing_field_attribute in &self.missing_field_attributes {
            if missing_field_attribute.field == field_name && missing_field_attribute.model.as_str() == model_name {
                Renderer::render_field_attribute(
                    &mut target.column_locked_writer_for(2),
                    &missing_field_attribute.attribute,
                )
            }
        }

        target.maybe_end_line();
    }

    fn reformat_type_alias(target: &mut TableFormat, token: &Token<'_>) {
        let mut identifier = None;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::TYPE_KEYWORD => {}
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    identifier = Some(String::from(current.as_str()))
                }
                Rule::base_type => {
                    target.write("type");
                    target.write(&identifier.clone().expect("Unknown field identifier."));
                    target.write("=");
                    target.write(Self::get_identifier(current));
                }
                Rule::attribute => {
                    Self::reformat_attribute(&mut target.column_locked_writer_for(4), &current, "@", vec![]);
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut target.interleave_writer(), current.as_str())
                }
                Rule::NEWLINE => {}
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        target.maybe_end_line();
    }

    fn reformat_field_type(token: &Token<'_>) -> String {
        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::optional_type => {
                    builder.write(Self::get_identifier(current));
                    builder.write("?");
                }
                Rule::base_type => {
                    builder.write(Self::get_identifier(current));
                }
                Rule::list_type => {
                    builder.write(Self::get_identifier(current));
                    builder.write("[]");
                }
                _ => Self::reformat_generic_token(&mut builder, &current),
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

    fn reformat_attribute(
        target: &mut dyn LineWriteable,
        token: &Token<'_>,
        owl: &str,
        missing_args: Vec<&MissingRelationAttributeArg>,
    ) {
        let token = Self::unpack_token_to_find_matching_rule(token.clone(), Rule::attribute);
        let mut is_relation = false;
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::attribute_name => {
                    if !target.line_empty() {
                        target.write(" ");
                    }
                    target.write(owl);
                    if current.as_str() == "relation" {
                        is_relation = true;
                    }
                    target.write(current.as_str());
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                Rule::arguments_list => {
                    if is_relation {
                        Self::reformat_arguments_list(target, &current, missing_args.as_slice())
                    } else {
                        Self::reformat_arguments_list(target, &current, &[])
                    }
                }
                Rule::NEWLINE => {}
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

    fn reformat_arguments_list(
        target: &mut dyn LineWriteable,
        token: &Token<'_>,
        missing_args: &[&MissingRelationAttributeArg],
    ) {
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
                _ => Self::reformat_generic_token(target, &current),
            };
        }

        if !missing_args.is_empty() {
            for arg in missing_args {
                if !builder.line_empty() {
                    builder.write(", ");
                }
                if let Some(arg_name) = &arg.arg.name {
                    builder.write(&arg_name.name);
                    builder.write(": ");
                }
                Self::render_value(&mut builder, &arg.arg.value);
            }
        }

        target.write("(");
        target.write(&builder.to_string());
        target.write(")");
    }

    //duplicated from renderer -.-
    fn render_value(target: &mut StringBuilder, val: &ast::Expression) {
        match val {
            ast::Expression::Array(vals, _) => Self::render_expression_array(target, vals),
            ast::Expression::ConstantValue(val, _) => target.write(val),
            ast::Expression::NumericValue(val, _) => target.write(val),
            ast::Expression::StringValue(val, _) => Self::render_str(target, val),
            ast::Expression::Function(name, args, _) => Self::render_func(target, name, args),
        };
    }

    fn render_argument(target: &mut StringBuilder, arg: &ast::Argument) {
        if let Some(arg_name) = &arg.name {
            target.write(&arg_name.name);
            target.write(": ");
        }

        Self::render_value(target, &arg.value);
    }

    fn render_expression_array(target: &mut StringBuilder, vals: &[ast::Expression]) {
        target.write("[");
        for (idx, arg) in vals.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }
            Self::render_value(target, arg);
        }
        target.write("]");
    }

    fn render_func(target: &mut StringBuilder, name: &str, args: &ast::ArgumentsList) {
        target.write(name);
        target.write("(");
        for (idx, arg) in args.arguments.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }

            Self::render_argument(target, arg);
        }
        target.write(")");
    }

    fn render_str(target: &mut StringBuilder, param: &str) {
        target.write("\"");
        target.write(&param.replace('\\', r#"\\"#).replace('"', r#"\""#).replace('\n', "\\n"));
        target.write("\"");
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
                Rule::arguments_list => Self::reformat_arguments_list(target, &current, &[]),
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
struct MissingField {
    model: String,
    field: ast::Field,
}

#[derive(Debug)]
struct MissingFieldAttribute {
    model: String,
    field: String,
    attribute: ast::Attribute,
}

#[derive(Debug)]
struct MissingRelationAttributeArg {
    model: String,
    field: String,
    arg: ast::Argument,
}

fn find_all_missing_relation_attribute_args(db: &ParserDatabase) -> Vec<MissingRelationAttributeArg> {
    let mut missing_relation_attribute_args = Vec::new();

    for relation in db.walk_relations() {
        match relation.refine() {
            walkers::RefinedRelationWalker::Inline(inline_relation) => {
                push_inline_relation_missing_arguments(inline_relation, &mut missing_relation_attribute_args)
            }
            walkers::RefinedRelationWalker::ImplicitManyToMany(_) => (),
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => (),
        }
    }

    missing_relation_attribute_args
}

fn push_inline_relation_missing_arguments(
    inline_relation: walkers::InlineRelationWalker<'_>,
    args: &mut Vec<MissingRelationAttributeArg>,
) {
    if let Some(forward) = inline_relation.forward_relation_field() {
        // the `fields: [...]` argument.
        match inline_relation.referencing_fields() {
            walkers::ReferencingFields::Concrete(_) => (),
            walkers::ReferencingFields::NA => (), // error somewhere else
            walkers::ReferencingFields::Inferred(fields) => {
                let missing_arg = MissingRelationAttributeArg {
                    model: forward.model().name().to_owned(),
                    field: forward.ast_field().name.name.to_owned(),
                    arg: ast::Argument {
                        name: Some(ast::Identifier::new("fields")),
                        value: ast::Expression::Array(
                            fields
                                .into_iter()
                                .map(|f| ast::Expression::ConstantValue(f.name, Span::empty()))
                                .collect(),
                            Span::empty(),
                        ),
                        span: Span::empty(),
                    },
                };
                args.push(missing_arg);
            }
        }

        // the `references: [...]` argument
        if forward.referenced_fields().is_none() {
            let missing_arg = MissingRelationAttributeArg {
                model: forward.model().name().to_owned(),
                field: forward.ast_field().name.name.to_owned(),
                arg: ast::Argument {
                    name: Some(ast::Identifier::new("references")),
                    value: ast::Expression::Array(
                        inline_relation
                            .referenced_fields()
                            .map(|f| ast::Expression::ConstantValue(f.name().to_owned(), Span::empty()))
                            .collect(),
                        Span::empty(),
                    ),
                    span: Span::empty(),
                },
            };
            args.push(missing_arg);
        }
    }
}

fn find_all_missing_attributes(db: &ParserDatabase) -> Vec<MissingFieldAttribute> {
    let mut missing_field_attributes = Vec::new();
    for relation in db.walk_relations() {
        if let walkers::RefinedRelationWalker::Inline(inline_relation) = relation.refine() {
            push_missing_relation_attribute(inline_relation, &mut missing_field_attributes);
        }
    }
    push_missing_unique_attributes(db, &mut missing_field_attributes);

    missing_field_attributes
}

fn push_missing_unique_attributes(db: &ParserDatabase, attributes: &mut Vec<MissingFieldAttribute>) {
    // Missing `@unique`s.
    let missing_unique_indexes = db
        .walk_models()
        .flat_map(|model| model.indexes())
        .filter(|idx| idx.ast_attribute().is_none());

    for missing_unique in missing_unique_indexes {
        if let Some(field) = missing_unique.source_field() {
            attributes.push(MissingFieldAttribute {
                model: missing_unique.model().name().to_owned(),
                field: field.name().to_owned(),
                attribute: ast::Attribute {
                    name: ast::Identifier::new("unique"),
                    arguments: ast::ArgumentsList::default(),
                    span: Span::empty(),
                },
            })
        }
    }
}

fn push_missing_relation_attribute(
    inline_relation: walkers::InlineRelationWalker<'_>,
    missing_attributes: &mut Vec<MissingFieldAttribute>,
) {
    if let Some(forward) = inline_relation.forward_relation_field() {
        if forward.relation_attribute().is_some() {
            return;
        }

        // the `fields: [...]` argument.
        let fields: Option<ast::Argument> = match inline_relation.referencing_fields() {
            walkers::ReferencingFields::Concrete(_) => None,
            walkers::ReferencingFields::NA => None, // error somewhere else
            walkers::ReferencingFields::Inferred(fields) => Some(ast::Argument {
                name: Some(ast::Identifier::new("fields")),
                value: ast::Expression::Array(
                    fields
                        .into_iter()
                        .map(|f| ast::Expression::ConstantValue(f.name, Span::empty()))
                        .collect(),
                    Span::empty(),
                ),
                span: Span::empty(),
            }),
        };

        // the `references: [...]` argument
        let references: Option<ast::Argument> = if forward.referenced_fields().is_none() {
            Some(ast::Argument {
                name: Some(ast::Identifier::new("references")),
                value: ast::Expression::Array(
                    inline_relation
                        .referenced_fields()
                        .map(|f| ast::Expression::ConstantValue(f.name().to_owned(), Span::empty()))
                        .collect(),
                    Span::empty(),
                ),
                span: Span::empty(),
            })
        } else {
            None
        };

        if let (Some(fields), Some(references)) = (fields, references) {
            missing_attributes.push(MissingFieldAttribute {
                model: forward.model().name().to_owned(),
                field: forward.name().to_owned(),
                attribute: ast::Attribute {
                    name: ast::Identifier::new("relation"),
                    arguments: ast::ArgumentsList {
                        arguments: vec![fields, references],
                        empty_arguments: Vec::new(),
                        trailing_comma: None,
                    },
                    span: Span::empty(),
                },
            })
        }
    }
}

// this finds all auto generated fields, that are added during auto generation AND are missing from the original input.
fn find_all_missing_fields(db: &ParserDatabase) -> Vec<MissingField> {
    let mut result = Vec::new();

    for relation in db.walk_relations() {
        if let Some(inline) = relation.refine().as_inline() {
            push_missing_fields(inline, &mut result);
        }
    }

    result
}

fn push_missing_fields(relation: walkers::InlineRelationWalker<'_>, missing_fields: &mut Vec<MissingField>) {
    push_missing_relation_fields(relation, missing_fields);
    push_missing_scalar_fields(relation, missing_fields);
}

fn push_missing_relation_fields(inline: walkers::InlineRelationWalker<'_>, missing_fields: &mut Vec<MissingField>) {
    if inline.back_relation_field().is_none() {
        let mut attributes = Vec::new();

        if inline.referencing_model().is_ignored() {
            attributes.push(ast::Attribute {
                name: ast::Identifier::new("ignore"),
                arguments: Default::default(),
                span: Span::empty(),
            })
        }

        missing_fields.push(MissingField {
            model: inline.referenced_model().name().to_owned(),
            field: ast::Field {
                field_type: ast::FieldType::Supported(ast::Identifier::new(inline.referencing_model().name())),
                name: ast::Identifier::new(inline.referencing_model().name()),
                arity: ast::FieldArity::List,
                attributes,
                documentation: None,
                span: Span::empty(),
                is_commented_out: false,
            },
        })
    }

    if inline.forward_relation_field().is_none() {
        missing_fields.push(MissingField {
            model: inline.referencing_model().name().to_owned(),
            field: ast::Field {
                field_type: ast::FieldType::Supported(ast::Identifier::new(inline.referenced_model().name())),
                name: ast::Identifier::new(inline.referenced_model().name()),
                arity: inline.forward_relation_field_arity(),
                attributes: vec![ast::Attribute {
                    name: ast::Identifier::new("relation"),
                    arguments: ast::ArgumentsList {
                        arguments: vec![
                            ast::Argument {
                                name: Some(ast::Identifier::new("fields")),
                                value: ast::Expression::Array(
                                    match inline.referencing_fields() {
                                        walkers::ReferencingFields::Concrete(fields) => fields
                                            .map(|f| ast::Expression::ConstantValue(f.name().to_owned(), Span::empty()))
                                            .collect(),
                                        walkers::ReferencingFields::Inferred(fields) => fields
                                            .into_iter()
                                            .map(|f| ast::Expression::ConstantValue(f.name, Span::empty()))
                                            .collect(),
                                        walkers::ReferencingFields::NA => Vec::new(),
                                    },
                                    Span::empty(),
                                ),
                                span: Span::empty(),
                            },
                            ast::Argument {
                                name: Some(ast::Identifier::new("references")),
                                value: ast::Expression::Array(
                                    inline
                                        .referenced_fields()
                                        .map(|f| ast::Expression::ConstantValue(f.name().to_owned(), Span::empty()))
                                        .collect(),
                                    Span::empty(),
                                ),
                                span: Span::empty(),
                            },
                        ],
                        empty_arguments: Vec::new(),
                        trailing_comma: None,
                    },
                    span: Span::empty(),
                }],
                documentation: None,
                span: Span::empty(),
                is_commented_out: false,
            },
        })
    }
}

fn push_missing_scalar_fields(inline: walkers::InlineRelationWalker<'_>, missing_fields: &mut Vec<MissingField>) {
    let missing_scalar_fields = match inline.referencing_fields() {
        walkers::ReferencingFields::Inferred(inferred_fields) => inferred_fields,
        _ => return,
    };

    // Filter out duplicate fields
    let missing_scalar_fields = missing_scalar_fields.iter().filter(|missing| {
        !inline
            .referencing_model()
            .scalar_fields()
            .any(|sf| sf.name() == missing.name)
    });

    for field in missing_scalar_fields {
        let field_type = if let Some(ft) = field.tpe.as_builtin_scalar() {
            ft
        } else {
            return;
        };

        let mut attributes: Vec<ast::Attribute> = Vec::new();
        if let Some((datasource_name, type_name, _args, _span)) = field.blueprint.raw_native_type() {
            let expected_attr_name = format!("{datasource_name}.{type_name}");
            attributes.push(
                field
                    .blueprint
                    .ast_field()
                    .attributes
                    .iter()
                    .find(|attr| attr.name.name == expected_attr_name)
                    .unwrap()
                    .clone(),
            );
        }

        missing_fields.push(MissingField {
            model: inline.referencing_model().name().to_owned(),
            field: ast::Field {
                field_type: ast::FieldType::Supported(ast::Identifier::new(field_type.as_str())),
                name: ast::Identifier::new(&field.name),
                arity: inline.forward_relation_field_arity(),
                attributes,
                documentation: None,
                span: Span::empty(),
                is_commented_out: false,
            },
        })
    }
}
