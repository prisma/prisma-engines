use psl::parser_database::ScalarType;
use schema_connector::{ConnectorError, ConnectorResult};

#[derive(Debug, Default)]
pub(crate) struct ParsedSqlDoc<'a> {
    parameters: Vec<ParsedParameterDoc<'a>>,
    description: Option<&'a str>,
}

impl<'a> ParsedSqlDoc<'a> {
    fn add_parameter(&mut self, param: ParsedParameterDoc<'a>) -> ConnectorResult<()> {
        if self
            .parameters
            .iter()
            .any(|p| p.position == param.position || p.alias == param.alias)
        {
            return Err(ConnectorError::from_msg(
                "duplicate parameter (position or alias is already used)".to_string(),
            ));
        }

        self.parameters.push(param);

        Ok(())
    }

    fn set_description(&mut self, doc: Option<&'a str>) {
        self.description = doc;
    }

    pub(crate) fn get_param_at(&self, at: usize) -> Option<&ParsedParameterDoc<'a>> {
        self.parameters.iter().find(|p| p.position == Some(at))
    }

    pub(crate) fn get_param_by_alias(&self, alias: &str) -> Option<&ParsedParameterDoc<'a>> {
        self.parameters.iter().find(|p| p.alias == Some(alias))
    }

    pub(crate) fn documentation(&self) -> Option<&str> {
        self.description
    }
}

#[derive(Debug, Default)]
pub(crate) struct ParsedParameterDoc<'a> {
    alias: Option<&'a str>,
    typ: Option<ScalarType>,
    position: Option<usize>,
    documentation: Option<&'a str>,
}

impl<'a> ParsedParameterDoc<'a> {
    fn set_alias(&mut self, name: Option<&'a str>) {
        self.alias = name;
    }

    fn set_typ(&mut self, typ: Option<ScalarType>) {
        self.typ = typ;
    }

    fn set_position(&mut self, position: Option<usize>) {
        self.position = position;
    }

    fn set_documentation(&mut self, doc: Option<&'a str>) {
        self.documentation = doc;
    }

    fn is_empty(&self) -> bool {
        self.alias.is_none() && self.position.is_none() && self.typ.is_none() && self.documentation.is_none()
    }

    pub(crate) fn alias(&self) -> Option<&str> {
        self.alias
    }

    pub(crate) fn typ(&self) -> Option<&str> {
        self.typ.map(|typ| typ.as_str())
    }

    pub(crate) fn documentation(&self) -> Option<&str> {
        self.documentation
    }
}

#[derive(Debug, Clone, Copy)]
struct Input<'a>(&'a str);

impl<'a> Input<'a> {
    fn find(&self, pat: &[char]) -> Option<usize> {
        self.0.find(pat)
    }

    fn strip_prefix_char(&self, pat: char) -> Option<Self> {
        self.0.strip_prefix(pat).map(Self)
    }

    fn strip_prefix_str(&self, pat: &str) -> Option<Self> {
        self.0.strip_prefix(pat).map(Self)
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.0.starts_with(pat)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn move_from(&self, n: usize) -> Input<'a> {
        Self(&self.0[n..])
    }

    fn move_to(&self, n: usize) -> Input<'a> {
        Self(&self.0[..n])
    }

    fn move_between(&self, start: usize, end: usize) -> Input<'a> {
        Self(&self.0[start..end])
    }

    fn move_to_end(&self) -> Input<'a> {
        Self(&self.0[self.0.len()..])
    }

    fn trim_start(&self) -> Input<'a> {
        Self(self.0.trim_start())
    }

    fn trim_end(&self) -> Input<'a> {
        Self(self.0.trim_end())
    }

    fn take_until_pat_or_eol(&self, pat: &[char]) -> (Input<'a>, &'a str) {
        if let Some(end) = self.find(pat) {
            (self.move_from(end), self.move_to(end).inner())
        } else {
            (self.move_to_end(), self.inner())
        }
    }

    fn inner(&self) -> &'a str {
        self.0
    }
}

impl std::fmt::Display for Input<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[inline]
fn build_error(input: Input<'_>, msg: &str) -> ConnectorError {
    ConnectorError::from_msg(format!("SQL documentation parsing: {msg} at '{input}'."))
}

fn parse_typ_opt(input: Input<'_>) -> ConnectorResult<(Input<'_>, Option<ScalarType>)> {
    if let Some(start) = input.find(&['{']) {
        if let Some(end) = input.find(&['}']) {
            let typ = input.move_between(start + 1, end);

            if typ.is_empty() {
                return Err(build_error(input, "missing type (accepted types are: 'Int', 'BigInt', 'Float', 'Boolean', 'String', 'DateTime', 'Json', 'Bytes', 'Decimal')"));
            }

            match ScalarType::try_from_str(typ.inner(), false) {
                Some(st) => {
                    Ok((input.move_from(end + 1), Some(st)))
                }
                None => {
                    Err(build_error(
                        input,
                        &format!("invalid type: '{typ}' (accepted types are: 'Int', 'BigInt', 'Float', 'Boolean', 'String', 'DateTime', 'Json', 'Bytes', 'Decimal')"),
                    ))
                }
            }
        } else {
            Err(build_error(input, "missing closing bracket"))
        }
    } else {
        Ok((input, None))
    }
}

fn parse_position_opt(input: Input<'_>) -> ConnectorResult<(Input<'_>, Option<usize>)> {
    if let Some((param_input, param_pos)) = input
        .trim_start()
        .strip_prefix_char('$')
        .map(|input| input.take_until_pat_or_eol(&[':', ' ']))
    {
        match param_pos.parse::<usize>().map_err(|_| {
            build_error(
                input,
                &format!("invalid position. Expected a number found: {param_pos}"),
            )
        }) {
            Ok(param_pos) => Ok((param_input, Some(param_pos))),
            Err(err) => Err(err),
        }
    } else {
        Ok((input, None))
    }
}

fn parse_alias_opt(input: Input<'_>) -> ConnectorResult<(Input<'_>, Option<&'_ str>)> {
    if let Some((input, alias)) = input
        .trim_start()
        .strip_prefix_char(':')
        .map(|input| input.take_until_pat_or_eol(&[' ']))
    {
        Ok((input, Some(alias)))
    } else {
        Ok((input, None))
    }
}

fn parse_rest(input: Input<'_>) -> ConnectorResult<Option<&str>> {
    let input = input.trim_start();

    if input.is_empty() {
        return Ok(None);
    }

    Ok(Some(input.trim_end().inner()))
}

fn validate_param(param: &ParsedParameterDoc<'_>, input: Input<'_>) -> ConnectorResult<()> {
    if param.is_empty() {
        return Err(build_error(input, "invalid parameter: could not parse any information"));
    }

    if param.position.is_none() && param.alias().is_none() {
        return Err(build_error(input, "missing position or alias (eg: $1:alias)"));
    }

    Ok(())
}

fn parse_param(param_input: Input<'_>) -> ConnectorResult<ParsedParameterDoc<'_>> {
    let input = param_input.strip_prefix_str("@param").unwrap().trim_start();

    let (input, typ) = parse_typ_opt(input)?;
    let (input, position) = parse_position_opt(input)?;
    let (input, alias) = parse_alias_opt(input)?;
    let documentation = parse_rest(input)?;

    let mut param = ParsedParameterDoc::default();

    param.set_typ(typ);
    param.set_position(position);
    param.set_alias(alias);
    param.set_documentation(documentation);

    validate_param(&param, param_input)?;

    Ok(param)
}

fn parse_description(input: Input<'_>) -> ConnectorResult<Option<&str>> {
    let input = input.strip_prefix_str("@description").unwrap();

    parse_rest(input)
}

pub(crate) fn parse_sql_doc(sql: &str) -> ConnectorResult<ParsedSqlDoc<'_>> {
    let mut parsed_sql = ParsedSqlDoc::default();

    let lines = sql.lines().peekable();

    for line in lines {
        let input = Input(line.trim());

        if let Some(input) = input.strip_prefix_str("--") {
            let input = input.trim_start();

            if input.starts_with("@description") {
                parsed_sql.set_description(parse_description(input)?);
            } else if input.starts_with("@param") {
                parsed_sql
                    .add_parameter(parse_param(input)?)
                    .map_err(|err| build_error(input, err.message().unwrap()))?;
            }
        }
    }

    Ok(parsed_sql)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_param_1() {
        use expect_test::expect;

        let res = parse_param(Input("@param $1:userId"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: Some(
                        "userId",
                    ),
                    typ: None,
                    position: Some(
                        1,
                    ),
                    documentation: None,
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_2() {
        use expect_test::expect;

        let res = parse_param(Input("@param $1:userId valid user identifier"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: Some(
                        "userId",
                    ),
                    typ: None,
                    position: Some(
                        1,
                    ),
                    documentation: Some(
                        "valid user identifier",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_3() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} :userId"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: Some(
                        "userId",
                    ),
                    typ: Some(
                        Int,
                    ),
                    position: None,
                    documentation: None,
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_4() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} $1:userId"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: Some(
                        "userId",
                    ),
                    typ: Some(
                        Int,
                    ),
                    position: Some(
                        1,
                    ),
                    documentation: None,
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_5() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} $1:userId valid user identifier"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: Some(
                        "userId",
                    ),
                    typ: Some(
                        Int,
                    ),
                    position: Some(
                        1,
                    ),
                    documentation: Some(
                        "valid user identifier",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_6() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} $1 valid user identifier"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: None,
                    typ: Some(
                        Int,
                    ),
                    position: Some(
                        1,
                    ),
                    documentation: Some(
                        "valid user identifier",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_7() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} $1f valid user identifier"));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: invalid position. Expected a number found: 1f at ' $1f valid user identifier'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: invalid position. Expected a number found: 1f at ' $1f valid user identifier'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_8() {
        use expect_test::expect;

        let res = parse_param(Input("@param {} valid user identifier"));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: missing type at '{} valid user identifier'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: missing type at '{} valid user identifier'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_9() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int $1f valid user identifier"));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: missing closing bracket at '{Int $1f valid user identifier'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: missing closing bracket at '{Int $1f valid user identifier'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_10() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int} valid user identifier $10"));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: missing position or alias (eg: $1:alias) at '@param {Int} valid user identifier $10'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: missing position or alias (eg: $1:alias) at '@param {Int} valid user identifier $10'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_11() {
        use expect_test::expect;

        let res = parse_param(Input("@param "));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: invalid parameter: could not parse any information at '@param '.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: invalid parameter: could not parse any information at '@param '.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_12() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Int}$1 some documentation"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: None,
                    typ: Some(
                        Int,
                    ),
                    position: Some(
                        1,
                    ),
                    documentation: Some(
                        "some documentation",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_13() {
        use expect_test::expect;

        let res = parse_param(Input("@param      {Int}        $1     some    documentation"));

        let expected = expect![[r#"
            Ok(
                ParsedParameterDoc {
                    alias: None,
                    typ: Some(
                        Int,
                    ),
                    position: Some(
                        1,
                    ),
                    documentation: Some(
                        "some    documentation",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_param_14() {
        use expect_test::expect;

        let res = parse_param(Input("@param {Unknown}  $1"));

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: invalid type: 'Unknown' (accepted types are: 'Int', 'BigInt', 'Float', 'Boolean', 'String', 'DateTime', 'Json', 'Bytes', 'Decimal') at '{Unknown}  $1'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: invalid type: 'Unknown' (accepted types are: 'Int', 'BigInt', 'Float', 'Boolean', 'String', 'DateTime', 'Json', 'Bytes', 'Decimal') at '{Unknown}  $1'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_sql_1() {
        use expect_test::expect;

        let res = parse_sql_doc("--   @param      {Int}        $1     some    documentation    ");

        let expected = expect![[r#"
            Ok(
                ParsedSqlDoc {
                    parameters: [
                        ParsedParameterDoc {
                            alias: None,
                            typ: Some(
                                Int,
                            ),
                            position: Some(
                                1,
                            ),
                            documentation: Some(
                                "some    documentation",
                            ),
                        },
                    ],
                    description: None,
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_sql_2() {
        use expect_test::expect;

        let res = parse_sql_doc(
            r#"     --     @description  This query returns a user by it's id
   --   @param   {Int}   $1:userId  valid   user identifier
  --         @param   {String}   $2:parentId  valid   parent identifier
    SELECT enum FROM "test_introspect_sql"."model"
        WHERE
            id = $1;"#,
        );

        let expected = expect![[r#"
            Ok(
                ParsedSqlDoc {
                    parameters: [
                        ParsedParameterDoc {
                            alias: Some(
                                "userId",
                            ),
                            typ: Some(
                                Int,
                            ),
                            position: Some(
                                1,
                            ),
                            documentation: Some(
                                "valid   user identifier",
                            ),
                        },
                        ParsedParameterDoc {
                            alias: Some(
                                "parentId",
                            ),
                            typ: Some(
                                String,
                            ),
                            position: Some(
                                2,
                            ),
                            documentation: Some(
                                "valid   parent identifier",
                            ),
                        },
                    ],
                    description: Some(
                        "This query returns a user by it's id",
                    ),
                },
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_sql_3() {
        use expect_test::expect;

        let res = parse_sql_doc(
            r#"--     @description  This query returns a user by it's id
    --   @param   {Int}      $1:userId  valid   user identifier
 --         @param   {String}   $1:parentId  valid   parent identifier
SELECT enum FROM "test_introspect_sql"."model" WHERE id = $1;"#,
        );

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param   {String}   $1:parentId  valid   parent identifier'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param   {String}   $1:parentId  valid   parent identifier'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }

    #[test]
    fn parse_sql_4() {
        use expect_test::expect;

        let res = parse_sql_doc(
            r#"--     @description  This query returns a user by it's id
--   @param   {Int}      $1:userId  valid   user identifier
--   @param   {String}   $2:userId  valid   parent identifier
SELECT enum FROM "test_introspect_sql"."model" WHERE id = $1;"#,
        );

        let expected = expect![[r#"
            Err(
                ConnectorErrorImpl {
                    user_facing_error: None,
                    message: Some(
                        "SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param   {String}   $2:userId  valid   parent identifier'.",
                    ),
                    source: None,
                    context: SpanTrace [],
                }
                SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param   {String}   $2:userId  valid   parent identifier'.
                ,
            )
        "#]];

        expected.assert_debug_eq(&res);
    }
}
