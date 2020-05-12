use crate::{ast::parser::*, ast::renderer::*};
use pest::Parser;

// We have to use RefCell as rust cannot
// do multiple mutable borrows inside a match statement.
use std::cell::RefCell;

type Token<'a> = pest::iterators::Pair<'a, Rule>;

pub struct ReformatterOld<'a> {
    input: &'a str,
    missing_back_relation_fields: Result<Vec<MissingField>, crate::error::ErrorCollection>,
}

fn count_lines(text: &str) -> usize {
    bytecount::count(text.as_bytes(), b'\n')
}

fn newlines(target: &mut dyn LineWriteable, text: &str, _identifier: &str) {
    for _i in 0..count_lines(text) {
        // target.write(&format!("{}{}", i, identifier));
        target.end_line();
    }
}

fn comment(target: &mut dyn LineWriteable, comment_text: &str) {
    let trimmed = if comment_text.ends_with("\n") {
        &comment_text[0..comment_text.len() - 1] // slice away line break.
    } else {
        &comment_text
    };

    if !target.line_empty() {
        // Prefix with whitespace seperator.
        target.write(&format!(" {}", trimmed));
    } else {
        target.write(trimmed);
    }
    target.end_line();
}

impl<'a> ReformatterOld<'a> {
    pub fn new(input: &'a str) -> Self {
        let missing_back_relation_fields = Self::find_all_missing_fields(&input);
        ReformatterOld {
            input,
            missing_back_relation_fields,
        }
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
        let mut ast = PrismaDatamodelParser::parse(Rule::datamodel, self.input).unwrap(); // TODO: Handle error.
        let mut top_formatter = RefCell::new(Renderer::new(output, ident_width));
        self.reformat_top(&mut top_formatter, &ast.next().unwrap());
    }

    fn reformat_top(&self, target: &mut RefCell<Renderer>, token: &Token) {
        let mut types_table = TableFormat::new();
        let mut types_mode = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {}
                Rule::type_declaration => {
                    types_mode = true;
                }
                _ => {
                    if types_mode {
                        types_mode = false;
                        // For all other ones, reset types_table.
                        types_table.render(target.get_mut());
                        types_table = TableFormat::new();
                        target.get_mut().maybe_end_line();
                    }
                }
            };
            match current.as_rule() {
                Rule::WHITESPACE => {
                    if types_mode {
                        let lines = count_lines(current.as_str());

                        if lines > 1 || (lines == 1 && types_table.line_empty()) {
                            // Reset the table layout on more than one newline.
                            types_table.render(target.get_mut());
                            types_table = TableFormat::new();
                        }

                        newlines(&mut types_table, current.as_str(), "m");
                    } else {
                        newlines(target.get_mut(), current.as_str(), "d")
                    }
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    if types_mode {
                        comment(&mut types_table.interleave_writer(), current.as_str());
                    } else {
                        comment(target.get_mut(), current.as_str());
                    }
                }
                Rule::model_declaration => self.reformat_model(target, &current),
                Rule::enum_declaration => Self::reformat_enum(target, &current),
                Rule::source_block => Self::reformat_config_block(target.get_mut(), &current),
                Rule::generator_block => Self::reformat_config_block(target.get_mut(), &current),
                Rule::type_declaration => {
                    if !types_mode {
                        panic!("Renderer not in type mode.");
                    }
                    Self::reformat_type_declaration(&mut types_table, &current);
                }
                Rule::EOI => {}
                _ => unreachable!(
                    "Encounterd impossible datamodel declaration during parsing: {:?}",
                    current.tokens()
                ),
            }
        }
    }

    fn reformat_config_block(target: &mut Renderer, token: &Token) {
        let mut table = TableFormat::new();
        // Switch to skip whitespace in 'datasource xxxx {'
        let mut skip_whitespace = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::GENERATOR_KEYWORD => {
                    skip_whitespace = true;
                    target.write("generator ");
                }
                Rule::DATASOURCE_KEYWORD => {
                    skip_whitespace = true;
                    target.write("datasource ");
                }
                Rule::BLOCK_OPEN => {
                    skip_whitespace = false;
                    target.write(" {");
                    target.maybe_end_line();
                    target.indent_up();
                }
                Rule::BLOCK_CLOSE => {}
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => target.write(current.as_str()),
                Rule::key_value => Self::reformat_key_value(&mut table, &current),
                Rule::doc_comment | Rule::doc_comment_and_new_line => comment(target, current.as_str()),
                Rule::WHITESPACE => {
                    if !skip_whitespace {
                        // TODO: This is duplicate.
                        let lines = count_lines(current.as_str());

                        if lines > 1 || (lines == 1 && table.line_empty()) {
                            // Reset the table layout on more than one newline.
                            table.render(target);
                            table = TableFormat::new();
                        }

                        newlines(&mut table, current.as_str(), "m");
                    }
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut table.interleave_writer(), current.as_str())
                }
                _ => unreachable!(
                    "Encounterd impossible source declaration during parsing: {:?}",
                    current.tokens()
                ),
            };
        }

        table.render(target);
        target.indent_down();
        target.write("}");
        target.maybe_end_line();
        target.maybe_end_line();
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside config key/value not supported yet.")
                }
                _ => unreachable!(
                    "Encounterd impossible source property declaration during parsing: {:?}",
                    current.tokens()
                ),
            }
        }
    }

    fn reformat_model(&self, target: &mut RefCell<Renderer>, token: &Token) {
        let mut table = RefCell::new(TableFormat::new());
        // Switch to skip whitespace in 'model xxxx {'
        let mut skip_whitespace = false;

        let mut model_name = "";

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::MODEL_KEYWORD => {
                    skip_whitespace = true;
                }
                Rule::BLOCK_OPEN => {
                    skip_whitespace = false;
                }
                Rule::BLOCK_CLOSE => {}

                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    // Begin.
                    model_name = current.as_str();
                    target.get_mut().write(&format!("model {} {{", model_name));
                    target.get_mut().maybe_end_line();
                    target.get_mut().indent_up();
                }
                Rule::directive => {
                    // Directives reset the table.
                    table.get_mut().render(target.get_mut());
                    table = RefCell::new(TableFormat::new());
                    Self::reformat_directive(target.get_mut(), &current, "@@");
                    table.get_mut().end_line();
                }
                Rule::field_declaration => Self::reformat_field(&mut table, &current),
                // Doc comments are to be placed OUTSIDE of table block.
                Rule::doc_comment | Rule::doc_comment_and_new_line => comment(target.get_mut(), current.as_str()),
                Rule::WHITESPACE => {
                    if !skip_whitespace {
                        let lines = count_lines(current.as_str());

                        if lines > 1 || (lines == 1 && table.get_mut().line_empty()) {
                            // Reset the table layout on more than one newline.
                            table.get_mut().render(target.get_mut());
                            table = RefCell::new(TableFormat::new());
                        }

                        newlines(table.get_mut(), current.as_str(), "m");
                    }
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut table.get_mut().interleave_writer(), current.as_str())
                }
                _ => unreachable!(
                    "Encounterd impossible model declaration during parsing: {:?}",
                    current.tokens()
                ),
            }
        }

        // TODO: what to do on error?
        for missing_back_relation_field in self.missing_back_relation_fields.as_ref().unwrap().iter() {
            if missing_back_relation_field.model == model_name {
                Renderer::render_field(table.get_mut(), &missing_back_relation_field.field, false);
            }
        }

        // End.
        table.get_mut().render(target.get_mut());
        target.get_mut().indent_down();
        target.get_mut().write("}");
        target.get_mut().maybe_end_line();
        target.get_mut().maybe_end_line();
    }

    // TODO: This is very similar to model reformating.
    fn reformat_enum(target: &mut RefCell<Renderer>, token: &Token) {
        let mut table = TableFormat::new();
        // Switch to skip whitespace in 'enum xxxx {'
        let mut skip_whitespace = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::ENUM_KEYWORD => {
                    skip_whitespace = true;
                }
                Rule::BLOCK_OPEN => {
                    skip_whitespace = false;
                }
                Rule::BLOCK_CLOSE => {}

                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    // Begin.
                    target.get_mut().write(&format!("enum {} {{", current.as_str()));
                    target.get_mut().maybe_end_line();
                    target.get_mut().indent_up();
                }
                Rule::directive => {
                    table.render(target.get_mut());
                    table = TableFormat::new();
                    Self::reformat_directive(target.get_mut(), &current, "@@");
                }
                Rule::enum_field_declaration => table.write(current.as_str()),
                // Doc comments are to be placed OUTSIDE of table block.
                Rule::doc_comment | Rule::doc_comment_and_new_line => comment(target.get_mut(), current.as_str()),
                Rule::WHITESPACE => {
                    if !skip_whitespace {
                        let lines = count_lines(current.as_str());

                        if lines > 1 || (lines == 1 && table.line_empty()) {
                            // Reset the table layout on more than one newline.
                            table.render(target.get_mut());
                            table = TableFormat::new();
                        }

                        newlines(&mut table, current.as_str(), "m");
                    }
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut table.interleave_writer(), current.as_str())
                }
                _ => unreachable!(
                    "Encounterd impossible enum declaration during parsing: {:?}",
                    current.tokens()
                ),
            }
        }

        // End.
        table.render(target.get_mut());
        target.get_mut().indent_down();
        target.get_mut().end_line();
        target.get_mut().write("\n}");
        target.get_mut().maybe_end_line();
        target.get_mut().maybe_end_line();
    }

    fn reformat_field(target: &mut RefCell<TableFormat>, token: &Token) {
        let mut identifier = None;
        let mut directives_started = false;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    identifier = Some(String::from(current.as_str()))
                }
                Rule::field_type => {
                    target
                        .get_mut()
                        .write(&identifier.clone().expect("Unknown field identifier."));
                    target.get_mut().write(&Self::reformat_field_type(&current));
                }
                Rule::directive => {
                    directives_started = true;
                    Self::reformat_directive(&mut target.get_mut().column_locked_writer_for(2), &current, "@")
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut target.get_mut().interleave_writer(), current.as_str())
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    if directives_started {
                        comment(&mut target.get_mut().column_locked_writer_for(2), current.as_str());
                    } else {
                        comment(target.get_mut(), current.as_str());
                    }
                }
                Rule::WHITESPACE => newlines(target.get_mut(), current.as_str(), "f"),
                _ => unreachable!("Encounterd impossible field during parsing: {:?}", current.tokens()),
            }
        }

        target.get_mut().maybe_end_line();
    }

    fn reformat_type_declaration(target: &mut TableFormat, token: &Token) {
        let mut identifier = None;
        let mut directives_started = false;

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
                    directives_started = true;
                    Self::reformat_directive(&mut target.column_locked_writer_for(4), &current, "@");
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    comment(&mut target.interleave_writer(), current.as_str())
                }
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    if directives_started {
                        comment(&mut target.column_locked_writer_for(4), current.as_str());
                    } else {
                        comment(&mut target.interleave_writer(), current.as_str());
                    }
                }
                Rule::WHITESPACE => newlines(target, current.as_str(), "t"),
                _ => unreachable!(
                    "Encounterd impossible custom type during parsing: {:?}",
                    current.tokens()
                ),
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
                _ => unreachable!(
                    "Encounterd impossible field type during parsing: {:?}",
                    current.tokens()
                ),
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                Rule::directive_arguments => Self::reformat_directive_args(target, &current),
                _ => unreachable!("Encounterd impossible directive during parsing: {:?}", current.tokens()),
            }
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attribute argument list not supported yet.")
                }
                _ => unreachable!(
                    "Encounterd impossible directive argument list during parsing: {:?}",
                    current.tokens()
                ),
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attribute argument not supported yet.")
                }
                _ => unreachable!(
                    "Encounterd impossible directive argument during parsing: {:?}",
                    current.tokens()
                ),
            };
        }
    }

    fn reformat_arg_value(target: &mut dyn LineWriteable, token: &Token) {
        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::expression => Self::reformat_expression(target, &current),
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside attributes not supported yet.")
                }
                _ => unreachable!(
                    "Encounterd impossible argument value during parsing: {:?}",
                    current.tokens()
                ),
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => unreachable!("Encounterd impossible literal during parsing: {:?}", current.tokens()),
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
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => unreachable!("Encounterd impossible array during parsing: {:?}", current.tokens()),
            }
        }

        target.write("]");
    }

    fn reformat_function_expression(target: &mut dyn LineWriteable, token: &Token) {
        let mut expr_count = 0;

        for current in token.clone().into_inner() {
            match current.as_rule() {
                Rule::non_empty_identifier | Rule::maybe_empty_identifier => {
                    target.write(current.as_str());
                    target.write("(");
                }
                Rule::argument_value => {
                    if expr_count > 0 {
                        target.write(", ");
                    }
                    Self::reformat_arg_value(target, &current);
                    expr_count += 1;
                }
                Rule::WHITESPACE => {}
                Rule::doc_comment | Rule::doc_comment_and_new_line => {
                    panic!("Comments inside expressions not supported yet.")
                }
                _ => unreachable!("Encounterd impossible function during parsing: {:?}", current.tokens()),
            }
        }

        target.write(")");
    }
}

#[derive(Debug)]
pub struct MissingField {
    pub model: String,
    pub field: crate::ast::Field,
}
