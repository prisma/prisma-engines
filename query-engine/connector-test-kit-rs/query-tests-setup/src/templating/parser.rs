use super::*;
use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::{tag, take_till, take_until},
    character::complete::{char, multispace0},
    error::{Error as NomError, ErrorKind},
    multi::{many0, separated_list0},
    sequence::delimited,
    IResult,
};
use parse_hyperlinks::take_until_unbalanced;

/// Main entry point into the template parsing. Parses a schema fragment of the form `#<fragment_ident>...<eol>`.
pub fn parse(fragment: &str) -> TemplatingResult<DatamodelFragment> {
    let (_, fragment) =
        parse_fragment(fragment).map_err(|err| TemplatingError::nom_error("unknown", err.to_string()))?;

    Ok(fragment)
}

// Todo: Error handling is a mess.
#[track_caller]
fn parse_fragment(input: &str) -> IResult<&str, DatamodelFragment> {
    let (input, _) = tag("#")(input)?;
    let (input, fragment_ident) = take_until("(")(input)?;

    // Produces the args string, e.g. "id, Int, @id"
    let (_, args) = unwrap_parenthesis(input)?;
    let (input, parsed_args) = many0(parse_fragment_argument)(args)?;

    let fragment = match DatamodelFragment::parse(fragment_ident, parsed_args) {
        Ok(fragment) => fragment,
        Err(err) => panic!("Invalid fragment definition '{fragment_ident}': {err}"),
    };

    Ok((input, fragment))
}

fn remove_whitespace<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: 'a + Fn(&'a str) -> IResult<&'a str, O, E>,
    E: nom::error::ParseError<&'a str>,
{
    delimited(multispace0, inner, multispace0)
}

fn unwrap_parenthesis(input: &str) -> IResult<&str, &str> {
    delimited(char('('), take_until_unbalanced('(', ')'), char(')'))(input)
}

fn parse_fragment_argument(input: &str) -> IResult<&str, FragmentArgument> {
    if input.is_empty() {
        return Err(nom::Err::Error(NomError::new(input, ErrorKind::NonEmpty)));
    }

    alt((parse_directive_argument, parse_value_argument))(input)
}

fn parse_directive_argument(input: &str) -> IResult<&str, FragmentArgument> {
    // Trim & discard `@`
    let (input, _) = remove_whitespace(tag("@"))(input)?;

    // Fragment arguments can have parenthesis and argument lists of their own,
    // so we need to find out what comes first: `(` or `,`.
    let (input, ident) = take_till(|c| c == '(' || c == ',')(input)?;
    if input.starts_with('(') {
        // `(` came first, parse argument parameters.
        let (input, all_args) = unwrap_parenthesis(input)?;

        // Todo: This will fail for @relation with nested commas (e.g. `fields: [field1, field2]`)
        let (_, chunked_args) = separated_list0(char(','), remove_whitespace(is_not(",")))(all_args)?;

        // Remove trailing comma, if any.
        let (input, _) = many0(remove_whitespace(char(',')))(input)?;

        Ok((input, FragmentArgument::Directive(Directive::new(ident, chunked_args))))
    } else {
        // `,` came first, remove it to allow parsing the next one.
        let (input, _) = many0(remove_whitespace(char(',')))(input)?;
        Ok((input, FragmentArgument::Directive(Directive::new(ident, vec![]))))
    }
}

fn parse_value_argument(input: &str) -> IResult<&str, FragmentArgument> {
    let (rest, arg) = remove_whitespace(take_till(|c| c == ','))(input)?;
    let (rest, _) = many0(remove_whitespace(char(',')))(rest)?;

    Ok((rest, FragmentArgument::Value(arg.to_owned())))
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    // Valid ID fragment
    fn basic_id_fragment_parsing() {
        let fragment = r#"#id(id, Int, @id, @map("_id"))"#;
        let fragment = parse_fragment(fragment);

        assert_eq!(
            fragment,
            Ok((
                "",
                DatamodelFragment::Id(IdFragment {
                    field_name: String::from("id"),
                    field_type: String::from("Int"),
                    directives: vec![
                        Directive {
                            ident: String::from("id"),
                            args: vec![]
                        },
                        Directive {
                            ident: String::from("map"),
                            args: vec![String::from("\"_id\"")]
                        }
                    ]
                })
            ))
        );
    }

    #[test]
    #[should_panic]
    // Invalid ID fragment
    fn no_args_id_fragment() {
        let fragment = r#"#id()"#;
        parse_fragment(fragment).unwrap();
    }

    #[test]
    fn valid_directive_arg() {
        let directive = r#"@map("_id")"#;
        let parsed = parse_fragment_argument(directive);

        assert_eq!(
            parsed,
            Ok((
                "",
                FragmentArgument::Directive(Directive {
                    ident: String::from("map"),
                    args: vec![String::from("\"_id\"")]
                })
            ))
        );
    }

    #[test]
    fn valid_value_arg() {
        let directive = r#"someString"#;
        let parsed = parse_fragment_argument(directive);

        assert_eq!(parsed, Ok(("", FragmentArgument::Value(String::from("someString")))));
    }

    #[test]
    // Valid m2m fragment
    fn basic_m2m_fragment_parsing() {
        let fragment = r#"#m2m(posts, Post[], id, String, some_name)"#;
        let fragment = parse_fragment(fragment);

        assert_eq!(
            fragment,
            Ok((
                "",
                DatamodelFragment::M2m(M2mFragment {
                    field_name: String::from("posts"),
                    field_type: String::from("Post[]"),
                    opposing_name: String::from("id"),
                    opposing_type: String::from("String"),
                    relation_name: Some(String::from("some_name")),
                })
            ))
        );
    }

    #[test]
    #[should_panic]
    fn invalid_m2m_fragment() {
        let fragment = r#"#m2m(name, Type)"#;

        parse_fragment(fragment).unwrap();
    }
}
