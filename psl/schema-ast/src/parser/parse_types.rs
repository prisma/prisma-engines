use super::{Rule, helpers::Pair};
use crate::{ast::*, parser::parse_expression::parse_expression};
use diagnostics::{DatamodelError, Diagnostics, FileId};

pub fn parse_field_type(
    pair: Pair<'_>,
    diagnostics: &mut Diagnostics,
    file_id: FileId,
) -> Result<(FieldArity, FieldType), DatamodelError> {
    assert!(pair.as_rule() == Rule::field_type);
    let current = pair.into_inner().next().unwrap();
    match current.as_rule() {
        Rule::optional_type => Ok((
            FieldArity::Optional,
            parse_base_type(current.into_inner().next().unwrap(), diagnostics, file_id),
        )),
        Rule::base_type => Ok((FieldArity::Required, parse_base_type(current, diagnostics, file_id))),
        Rule::list_type => Ok((
            FieldArity::List,
            parse_base_type(current.into_inner().next().unwrap(), diagnostics, file_id),
        )),
        Rule::legacy_required_type => Err(DatamodelError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            (file_id, current.as_span()).into(),
        )),
        Rule::legacy_list_type => Err(DatamodelError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            (file_id, current.as_span()).into(),
        )),
        Rule::unsupported_optional_list_type => Err(DatamodelError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            (file_id, current.as_span()).into(),
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens()),
    }
}

fn parse_base_type(pair: Pair<'_>, diagnostics: &mut Diagnostics, file_id: FileId) -> FieldType {
    let current = pair.into_inner().next().unwrap();
    match current.as_rule() {
        Rule::identifier => FieldType::Supported(Identifier {
            name: current.as_str().to_string(),
            span: Span::from((file_id, current.as_span())),
        }),
        Rule::unsupported_type => match parse_expression(current, diagnostics, file_id) {
            Expression::StringValue(lit, span) => FieldType::Unsupported(lit, span),
            _ => unreachable!("Encountered impossible type during parsing"),
        },
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
    }
}
