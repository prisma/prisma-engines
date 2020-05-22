use crate::{ast::parser::*, ast::renderer::*};
use pest::Parser;

// We have to use RefCell as rust cannot
// do multiple mutable borrows inside a match statement.
use super::helpers::*;
use crate::common::WritableString;

pub struct Reformatter<'a> {
    input: &'a str,
    missing_fields: Result<Vec<MissingField>, crate::error::ErrorCollection>,
}

impl<'a> Reformatter<'a> {
    pub fn new(input: &'a str) -> Self {
        let missing_fields = Self::find_all_missing_fields(&input);
        Reformatter { input, missing_fields }
    }

    // this finds all auto generated fields, that are added during auto generation AND are missing from the original input.
    fn find_all_missing_fields(schema_string: &str) -> Result<Vec<MissingField>, crate::error::ErrorCollection> {
        let schema_ast = crate::parse_schema_ast(&schema_string)?;
        let datamodel = crate::lift_ast(&schema_ast)?;
        let lowerer = crate::validator::LowerDmlToAst::new();
        let mut result = Vec::new();

        for model in datamodel.models() {
            let ast_model = schema_ast.find_model(&model.name).unwrap();

            for field in model.fields() {
                if ast_model.fields.iter().find(|f| &f.name.name == &field.name).is_none() {
                    let ast_field = lowerer.lower_field(&field, &datamodel)?;

                    result.push(MissingField {
                        model: model.name.clone(),
                        field: ast_field,
                    });
                }
            }
        }

        Ok(result)
    }

    pub fn reformat_to(&self, output: &mut dyn std::io::Write, ident_width: usize) {
        let result = self.reformat_internal(ident_width);
        write!(output, "{}", result).unwrap()
    }

    pub fn reformat_to_string(&self) -> String {
        self.reformat_internal(2)
    }

    fn reformat_internal(&self, ident_width: usize) -> String {
        let mut ast = PrismaDatamodelParser::parse(Rule::schema, self.input).unwrap(); // TODO: Handle error.
        let mut target_string = WritableString::new();
        let mut renderer = Renderer::new(&mut target_string, ident_width);
        self.reformat_top(&mut renderer, &ast.next().unwrap());
        let result = target_string.into();
        // all schemas must end with a newline
        if result.ends_with('\n') {
            result
        } else {
            format!("{}\n", result)
        }
    }

    fn reformat_top(&self, target: &mut Renderer, token: &Token) {
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
                    //                    println!("rendering newline");
                    target.write("\n");
                }
                seen_at_least_one_top_level_element = true;
            }

            //            println!("top level: {:?} |{:?}|", current.as_rule(), current.as_str());

            match current.as_rule() {
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    if types_mode {
                        comment(&mut types_table.interleave_writer(), current.as_str());
                    } else {
                        comment(target, current.as_str());
                    }
                }
                Rule::model_declaration => self.reformat_model(target, &current),
                Rule::enum_declaration => self.reformat_enum(target, &current),
                Rule::source_block => self.reformat_datasource(target, &current),
                Rule::generator_block => self.reformat_generator(target, &current),
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

    fn reformat_datasource(&self, target: &mut Renderer, token: &Token) {
        self.reformat_block_element(
            "datasource",
            target,
            token,
            Box::new(|table, _, token| match token.as_rule() {
                Rule::key_value => Self::reformat_key_value(table, &token),
                _ => Self::reformat_generic_token(table, &token),
            }),
        );
    }

    fn reformat_generator(&self, target: &mut Renderer, token: &Token) {
        self.reformat_block_element(
            "generator",
            target,
            token,
            Box::new(|table, _, token| {
                //
                match token.as_rule() {
                    Rule::key_value => Self::reformat_key_value(table, &token),
                    _ => Self::reformat_generic_token(table, &token),
                }
            }),
        );
    }

    fn reformat_key_value(target: &mut TableFormat, token: &Token) {
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

    fn reformat_model(&self, target: &mut Renderer, token: &Token) {
        self.reformat_block_element_internal(
            "model",
            target,
            &token,
            Box::new(|table, renderer, token| {
                match token.as_rule() {
                    Rule::directive => {
                        // model level Directives reset the table. -> .render() does that
                        table.render(renderer);
                        Self::reformat_directive(renderer, &token, "@@");
                    }
                    Rule::field_declaration => Self::reformat_field(table, &token),
                    _ => Self::reformat_generic_token(table, &token),
                }
            }),
            Box::new(|table, _, model_name| {
                // TODO: what is the right thing to do on error?
                if let Ok(missing_fields) = self.missing_fields.as_ref() {
                    for missing_back_relation_field in missing_fields.iter() {
                        if missing_back_relation_field.model.as_str() == model_name {
                            Renderer::render_field(table, &missing_back_relation_field.field, false);
                        }
                    }
                }
            }),
        );
    }

    fn reformat_block_element(
        &self,
        block_type: &'static str,
        renderer: &'a mut Renderer,
        token: &'a Token,
        the_fn: Box<dyn Fn(&mut TableFormat, &mut Renderer, &Token) -> () + 'a>,
    ) {
        self.reformat_block_element_internal(block_type, renderer, token, the_fn, {
            // a no op
            Box::new(|_, _, _| ())
        })
    }

    fn reformat_block_element_internal(
        &self,
        block_type: &'static str,
        renderer: &'a mut Renderer,
        token: &'a Token,
        the_fn: Box<dyn Fn(&mut TableFormat, &mut Renderer, &Token) -> () + 'a>,
        after_fn: Box<dyn Fn(&mut TableFormat, &mut Renderer, &str) -> () + 'a>,
    ) {
        let mut table = TableFormat::new();
        let mut block_name = "";
        let mut block_has_opened = false;

        for current in token.clone().into_inner() {
            //println!("block: {:?} |{:?}|", current.as_rule(), current.as_str());
            match current.as_rule() {
                Rule::BLOCK_OPEN => {
                    block_has_opened = true;
                }
                Rule::BLOCK_CLOSE => {}

                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    // Begin.
                    block_name = current.as_str();
                    renderer.write(&format!("{} {} {{", block_type, block_name));
                    renderer.maybe_end_line();
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
                        // do not render newlines before the block
                        // Reset the table layout on a newline.
                        table.render(renderer);
                        table = TableFormat::new();
                        renderer.end_line();
                    }
                }
                _ => the_fn(&mut table, renderer, &current),
            }
        }

        after_fn(&mut table, renderer, block_name);

        // End.
        table.render(renderer);
        renderer.indent_down();
        renderer.write("}");
        renderer.maybe_end_line();
    }

    fn reformat_enum(&self, target: &mut Renderer, token: &Token) {
        self.reformat_block_element(
            "enum",
            target,
            token,
            Box::new(|table, target, token| {
                //
                match token.as_rule() {
                    Rule::block_level_directive => {
                        table.render(target);
                        Self::reformat_directive(target, token, "@@");
                        table.end_line();
                    }
                    Rule::enum_value_declaration => Self::reformat_enum_entry(table, token),
                    _ => Self::reformat_generic_token(table, token),
                }
            }),
        );
    }

    fn reformat_enum_entry(target: &mut TableFormat, token: &Token) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier => target.write(current.as_str()),
                Rule::directive => Self::reformat_directive(&mut target.column_locked_writer_for(2), &current, "@"),
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn reformat_field(target: &mut TableFormat, token: &Token) {
        let mut identifier = None;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    identifier = Some(String::from(current.as_str()))
                }
                Rule::field_type => {
                    target.write(&identifier.clone().expect("Unknown field identifier."));
                    target.write(&Self::reformat_field_type(&current));
                }
                Rule::directive => Self::reformat_directive(&mut target.column_locked_writer_for(2), &current, "@"),
                // This is a comment at the end of a field.
                Rule::doc_comment => comment(target, current.as_str()),
                // This is a comment before the field declaration. Hence it must be interlevaed.
                Rule::doc_comment_and_new_line => comment(&mut target.interleave_writer(), current.as_str()),
                Rule::NEWLINE => {} // we do the new lines ourselves
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        target.maybe_end_line();
    }

    fn reformat_type_alias(target: &mut TableFormat, token: &Token) {
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
                    target.write(&Self::get_identifier(&current));
                }
                Rule::directive => {
                    Self::reformat_directive(&mut target.column_locked_writer_for(4), &current, "@");
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

    fn reformat_field_type(token: &Token) -> String {
        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            builder.write(&Self::get_identifier(&current));
            match current.as_rule() {
                Rule::optional_type => builder.write("?"),
                Rule::base_type => {}
                Rule::list_type => builder.write("[]"),
                _ => Self::reformat_generic_token(&mut builder, &current),
            }
        }

        builder.to_string()
    }

    fn get_identifier(token: &Token) -> String {
        for current in token.clone().into_inner() {
            if let Rule::non_empty_identifier | Rule::maybe_empty_identifier = current.as_rule() {
                return current.as_str().to_string();
            }
        }

        panic!("No identifier found.")
    }

    fn reformat_directive(target: &mut dyn LineWriteable, token: &Token, owl: &str) {
        let token = Self::unpack_token_to_find_matching_rule(token.clone(), Rule::directive);
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::directive_name => {
                    // Begin
                    if !target.line_empty() {
                        target.write(" ");
                    }
                    target.write(owl);
                    target.write(current.as_str());
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                Rule::directive_arguments => Self::reformat_directive_args(target, &current),
                Rule::NEWLINE => {}
                _ => Self::reformat_generic_token(target, &current),
            }
        }
    }

    fn unpack_token_to_find_matching_rule(token: Token, rule: Rule) -> Token {
        if token.as_rule() == rule {
            token
        } else {
            let error_msg = format!("Token matching rule {:?} not found in: {:?}", &rule, &token.as_str());
            for token in token.into_inner() {
                if token.as_rule() == rule {
                    return token;
                }
            }
            panic!(error_msg)
        }
    }

    fn reformat_directive_args(target: &mut dyn LineWriteable, token: &Token) {
        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            match current.as_rule() {
                // This is a named arg.
                Rule::argument => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_directive_arg(&mut builder, &current);
                }
                // This is a an unnamed arg.
                Rule::argument_value => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_arg_value(&mut builder, &current);
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attribute argument list not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            };
        }

        if !builder.line_empty() {
            target.write("(");
            target.write(&builder.to_string());
            target.write(")");
        }
    }

    fn reformat_directive_arg(target: &mut dyn LineWriteable, token: &Token) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::argument_name => {
                    target.write(current.as_str());
                    target.write(": ");
                }
                Rule::argument_value => Self::reformat_arg_value(target, &current),
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attribute argument not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            };
        }
    }

    fn reformat_arg_value(target: &mut dyn LineWriteable, token: &Token) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::expression => Self::reformat_expression(target, &current),
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            };
        }
    }

    /// Parses an expression, given a Pest parser token.
    fn reformat_expression(target: &mut dyn LineWriteable, token: &Token) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::numeric_literal => target.write(current.as_str()),
                Rule::string_literal => target.write(current.as_str()),
                Rule::boolean_literal => target.write(current.as_str()),
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

    fn reformat_array_expression(target: &mut dyn LineWriteable, token: &Token) {
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

    fn reformat_function_expression(target: &mut dyn LineWriteable, token: &Token) {
        let mut has_seen_one_argument = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    target.write(current.as_str());
                    target.write("(");
                }
                Rule::expression => {
                    if has_seen_one_argument {
                        target.write(", ");
                    }
                    Self::reformat_expression(target, &current);
                    has_seen_one_argument = true;
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        target.write(")");
    }

    fn reformat_generic_token(target: &mut dyn LineWriteable, token: &Token) {
        //        println!("generic token: |{:?}|", token.as_str());
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
pub struct MissingField {
    pub model: String,
    pub field: crate::ast::Field,
}
