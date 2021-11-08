use super::helpers::*;
use crate::ast::helper::get_sort_index_of_attribute;
use crate::ast::SchemaAst;
use crate::diagnostics::{ValidatedDatamodel, ValidatedMissingFields};
use crate::Datasource;
use crate::{ast, ast::parser::*, ast::renderer::*};
use enumflags2::BitFlags;
use pest::iterators::Pair;
use pest::Parser;

pub struct Reformatter<'a> {
    input: &'a str,
    missing_fields: Result<ValidatedMissingFields, crate::diagnostics::Diagnostics>,
    missing_field_attributes: Result<Vec<MissingFieldAttribute>, crate::diagnostics::Diagnostics>,
    missing_relation_attribute_args: Result<Vec<MissingRelationAttributeArg>, crate::diagnostics::Diagnostics>,
}

impl<'a> Reformatter<'a> {
    pub fn new(input: &'a str) -> Self {
        match (
            crate::parse_schema_ast(input),
            crate::parse_datamodel_for_formatter(input),
        ) {
            (Ok(schema_ast), Ok(validated_datamodel)) => {
                let datasource = crate::parse_configuration(input)
                    .ok()
                    .and_then(|mut config| config.subject.datasources.pop());

                let missing_fields =
                    Self::find_all_missing_fields(&schema_ast, &validated_datamodel, datasource.as_ref());

                let missing_field_attributes =
                    Self::find_all_missing_attributes(&schema_ast, &validated_datamodel, datasource.as_ref());

                let missing_relation_attribute_args = Self::find_all_missing_relation_attribute_args(
                    &schema_ast,
                    &validated_datamodel,
                    datasource.as_ref(),
                );

                Reformatter {
                    input,
                    missing_fields,
                    missing_field_attributes,
                    missing_relation_attribute_args,
                }
            }
            (Err(diagnostics), _) => Reformatter {
                input,
                missing_field_attributes: Err(diagnostics.clone()),
                missing_relation_attribute_args: Err(diagnostics.clone()),
                missing_fields: Err(diagnostics),
            },

            (Ok(_), Err(diagnostics)) => Reformatter {
                input,
                missing_field_attributes: Err(diagnostics.clone()),
                missing_relation_attribute_args: Err(diagnostics.clone()),
                missing_fields: Err(diagnostics),
            },
        }
    }

    // this finds all auto generated fields, that are added during auto generation AND are missing from the original input.
    fn find_all_missing_fields(
        schema_ast: &SchemaAst,
        validated_datamodel: &ValidatedDatamodel,
        datasource: Option<&Datasource>,
    ) -> Result<ValidatedMissingFields, crate::diagnostics::Diagnostics> {
        let mut diagnostics = crate::diagnostics::Diagnostics::new();
        let lowerer = crate::transform::dml_to_ast::LowerDmlToAst::new(datasource, BitFlags::empty());
        let mut result = Vec::new();

        diagnostics.append_warning_vec(validated_datamodel.warnings.clone());

        for model in validated_datamodel.subject.models() {
            let ast_model = schema_ast.find_model(&model.name).unwrap();

            for field in model.fields() {
                if !ast_model.fields.iter().any(|f| f.name.name == field.name()) {
                    let ast_field = lowerer.lower_field(model, field, &validated_datamodel.subject);

                    result.push(MissingField {
                        model: model.name.clone(),
                        field: ast_field,
                    });
                }
            }
        }

        Ok(ValidatedMissingFields {
            subject: result,
            warnings: diagnostics.warnings().to_owned(),
        })
    }

    fn find_all_missing_attributes(
        schema_ast: &SchemaAst,
        validated_datamodel: &ValidatedDatamodel,
        datasource: Option<&Datasource>,
    ) -> Result<Vec<MissingFieldAttribute>, crate::diagnostics::Diagnostics> {
        let mut diagnostics = crate::diagnostics::Diagnostics::new();

        diagnostics.append_warning_vec(validated_datamodel.warnings.clone());
        let lowerer = crate::transform::dml_to_ast::LowerDmlToAst::new(datasource, BitFlags::empty());

        let mut missing_field_attributes = Vec::new();
        for model in validated_datamodel.subject.models() {
            let ast_model = schema_ast.find_model(&model.name).unwrap();
            for field in model.fields() {
                let new_ast_field = lowerer.lower_field(model, field, &validated_datamodel.subject);

                if let Some(original_field) = ast_model.fields.iter().find(|f| f.name.name == field.name()) {
                    for attribute in new_ast_field.attributes {
                        if !original_field
                            .attributes
                            .iter()
                            .any(|d| d.name.name == attribute.name.name)
                        {
                            missing_field_attributes.push(MissingFieldAttribute {
                                model: model.name.clone(),
                                field: field.name().to_string(),
                                attribute,
                            })
                        }
                    }
                }
            }
        }
        Ok(missing_field_attributes)
    }

    fn find_all_missing_relation_attribute_args(
        schema_ast: &SchemaAst,
        validated_datamodel: &ValidatedDatamodel,
        datasource: Option<&Datasource>,
    ) -> Result<Vec<MissingRelationAttributeArg>, crate::diagnostics::Diagnostics> {
        let mut diagnostics = crate::diagnostics::Diagnostics::new();

        diagnostics.append_warning_vec(validated_datamodel.warnings.clone());
        let lowerer = crate::transform::dml_to_ast::LowerDmlToAst::new(datasource, BitFlags::empty());

        let mut missing_relation_attribute_args = Vec::new();
        for model in validated_datamodel.subject.models() {
            let ast_model = schema_ast.find_model(&model.name).unwrap();
            for field in model.fields() {
                let new_ast_field = lowerer.lower_field(model, field, &validated_datamodel.subject);

                if let Some(original_field) = ast_model.fields.iter().find(|f| f.name.name == field.name()) {
                    for attribute in new_ast_field.attributes.iter().filter(|a| a.name.name == "relation") {
                        if let Some(original_attribute) = original_field
                            .attributes
                            .iter()
                            .find(|d| d.name.name == attribute.name.name)
                        {
                            for arg in &attribute.arguments {
                                if !arg.name.name.is_empty()
                                    && !original_attribute
                                        .arguments
                                        .iter()
                                        .any(|d| d.name.name == arg.name.name)
                                {
                                    missing_relation_attribute_args.push(MissingRelationAttributeArg {
                                        model: model.name.clone(),
                                        field: field.name().to_string(),
                                        arg: arg.to_owned(),
                                    })
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(missing_relation_attribute_args)
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
        let mut target_string = String::with_capacity(self.input.len());
        let mut renderer = Renderer::new(&mut target_string, ident_width);
        self.reformat_top(&mut renderer, &ast.next().unwrap());

        // all schemas must end with a newline
        if !target_string.ends_with('\n') {
            target_string.push('\n');
        }

        target_string
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

    fn reformat_datasource(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element(
            "datasource",
            target,
            token,
            &(|table, _, token, _| match token.as_rule() {
                Rule::key_value => Self::reformat_key_value(table, token),
                _ => Self::reformat_generic_token(table, token),
            }),
        );
    }

    fn reformat_generator(&self, target: &mut Renderer<'_>, token: &Token<'_>) {
        self.reformat_block_element(
            "generator",
            target,
            token,
            &(|table, _, token, _| {
                //
                match token.as_rule() {
                    Rule::key_value => Self::reformat_key_value(table, token),
                    _ => Self::reformat_generic_token(table, token),
                }
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
                // TODO: what is the right thing to do on error?
                if let Ok(missing_fields) = self.missing_fields.as_ref() {
                    for missing_back_relation_field in missing_fields.subject.iter() {
                        if missing_back_relation_field.model.as_str() == model_name {
                            Renderer::render_field(table, &missing_back_relation_field.field, false);
                        }
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
        block_type: &'static str,
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
        block_type: &'static str,
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
                Rule::MODEL_KEYWORD | Rule::TYPE_KEYWORD => (),
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
                //todo special case field attribute to pass model and field name and probably attribute name  and down to the args
                Rule::attribute => {
                    if let Ok(missing_relation_attribute_args) = self.missing_relation_attribute_args.as_ref() {
                        let missing_relation_args: Vec<&MissingRelationAttributeArg> = missing_relation_attribute_args
                            .iter()
                            .filter(|arg| arg.model == model_name && arg.field == *field_name)
                            .collect();

                        Self::reformat_attribute(
                            &mut target.column_locked_writer_for(2),
                            &current,
                            "@",
                            missing_relation_args,
                        )
                    } else {
                        Self::reformat_attribute(&mut target.column_locked_writer_for(2), &current, "@", vec![])
                    }
                }
                // This is a comment at the end of a field.
                Rule::doc_comment | Rule::comment => target.append_suffix_to_current_row(current.as_str()),
                // This is a comment before the field declaration. Hence it must be interlevaed.
                Rule::doc_comment_and_new_line => comment(&mut target.interleave_writer(), current.as_str()),
                Rule::NEWLINE => {} // we do the new lines ourselves
                _ => Self::reformat_generic_token(target, &current),
            }
        }

        if let Ok(missing_field_attributes) = self.missing_field_attributes.as_ref() {
            for missing_field_attribute in missing_field_attributes.iter() {
                if missing_field_attribute.field == field_name && missing_field_attribute.model.as_str() == model_name {
                    Renderer::render_field_attribute(
                        &mut target.column_locked_writer_for(2),
                        &missing_field_attribute.attribute,
                    )
                }
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
                Rule::attribute_arguments => {
                    if is_relation {
                        Self::reformat_attribute_args(target, &current, missing_args.clone())
                    } else {
                        Self::reformat_attribute_args(target, &current, vec![])
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

    fn reformat_attribute_args(
        target: &mut dyn LineWriteable,
        token: &Token<'_>,
        missing_args: Vec<&MissingRelationAttributeArg>,
    ) {
        let mut builder = StringBuilder::new();

        for current in token.clone().into_inner() {
            match current.as_rule() {
                // This is a named arg.
                Rule::argument => {
                    if !builder.line_empty() {
                        builder.write(", ");
                    }
                    Self::reformat_attribute_arg(&mut builder, &current);
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

        if !missing_args.is_empty() {
            for arg in missing_args {
                if !builder.line_empty() {
                    builder.write(", ");
                }
                builder.write(&arg.arg.name.name);
                builder.write(": ");
                Self::render_value(&mut builder, &arg.arg.value);
            }
        }

        if !builder.line_empty() {
            target.write("(");
            target.write(&builder.to_string());
            target.write(")");
        }
    }

    //duplicated from renderer -.-
    fn render_value(target: &mut StringBuilder, val: &ast::Expression) {
        match val {
            ast::Expression::Array(vals, _) => Self::render_expression_array(target, vals),
            ast::Expression::FieldWithArgs(ident, vals, _) => Self::render_constant_value_w_args(target, ident, vals),
            ast::Expression::BooleanValue(val, _) => target.write(val),
            ast::Expression::ConstantValue(val, _) => target.write(val),
            ast::Expression::NumericValue(val, _) => target.write(val),
            ast::Expression::StringValue(val, _) => Self::render_str(target, val),
            ast::Expression::Function(name, args, _) => Self::render_func(target, name, args),
        };
    }

    fn render_constant_value_w_args(target: &mut StringBuilder, ident: &str, vals: &[ast::Argument]) {
        target.write(ident);
        target.write("(");
        for (idx, arg) in vals.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }
            Self::render_argument(target, arg);
        }
        target.write(")");
    }

    fn render_argument(target: &mut StringBuilder, arg: &ast::Argument) {
        if !arg.name.name.is_empty() {
            target.write(&arg.name.name);
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
    fn render_func(target: &mut StringBuilder, name: &str, vals: &[ast::Expression]) {
        target.write(name);
        target.write("(");
        for (idx, val) in vals.iter().enumerate() {
            if idx > 0 {
                target.write(", ");
            }

            Self::render_value(target, val);
        }
        target.write(")");
    }

    fn render_str(target: &mut StringBuilder, param: &str) {
        target.write("\"");
        target.write(
            &param
                .replace(r#"\"#, r#"\\"#)
                .replace(r#"""#, r#"\""#)
                .replace("\n", "\\n"),
        );
        target.write("\"");
    }
    //duplicated from renderer -.-

    fn reformat_attribute_arg(target: &mut dyn LineWriteable, token: &Token<'_>) {
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

    fn reformat_arg_value(target: &mut dyn LineWriteable, token: &Token<'_>) {
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
    fn reformat_expression(target: &mut dyn LineWriteable, token: &Token<'_>) {
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
pub struct MissingField {
    pub model: String,
    pub field: crate::ast::Field,
}

#[derive(Debug)]
pub struct MissingFieldAttribute {
    pub model: String,
    pub field: String,
    pub attribute: crate::ast::Attribute,
}

#[derive(Debug)]
pub struct MissingRelationAttributeArg {
    pub model: String,
    pub field: String,
    pub arg: crate::ast::Argument,
}
