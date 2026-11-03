use crate::{
    ast::{NativeColumnType, OpaqueType, Query, Value, ValueType},
    error::{Error, ErrorKind},
    visitor::Mysql,
};
use query_template::{Fragment, PlaceholderFormat, QueryTemplate};
use std::borrow::Cow;

/// A MySQL-compatible SQL visitor for KingbaseES.
///
/// It preserves the MySQL visitor's SQL semantics, adding only the metadata
/// and casts required by Kingbase's `jsonpath` function arguments.
pub struct KingbaseMysql;

impl KingbaseMysql {
    pub fn build_template<'a, Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let template = Mysql::build_template_for_kingbase(query)?;
        let QueryTemplate {
            fragments,
            mut parameters,
            placeholder_format,
        } = template;
        let (sql, parameter_fragments) = template_sql(fragments);

        let sql = rewrite_full_text_search(sql, &parameters)?;
        let sql = cast_sum_parameters(sql, &parameters);
        promote_json_numeric_comparison_params(&sql, &mut parameters);
        let sql = cast_json_comparison_expressions(sql);

        for index in json_extract_path_parameter_indexes(&sql) {
            if let Some(param) = parameters.get_mut(index) {
                param.native_column_type = Some(NativeColumnType {
                    name: Cow::Borrowed("JSONPATH"),
                    length: None,
                });
            }
        }

        let sql = cast_dynamic_jsonpaths(sql);
        let sql = qualify_json_contains(sql);
        let sql = unquote_json_values(sql);
        let sql = native_uuid_as_text(sql);

        rebuild_template(sql, parameter_fragments, parameters, placeholder_format)
    }

    pub fn build<'a, Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>,
    {
        let template = Self::build_template(query)?;
        let sql = template.to_sql().map_err(|_| {
            Error::builder(ErrorKind::conversion(
                "Kingbase MySQL query contains dynamic parameter fragments",
            ))
            .build()
        })?;
        let params = template.parameters;
        Ok((sql, params))
    }
}

/// Rewrites MySQL MATCH SQL to Kingbase full-text functions while preserving
/// search parameters verbatim. Both Kingbase providers accept native tsquery
/// expressions. The fixed `simple` configuration matches MySQL-mode GIN indexes.
fn rewrite_full_text_search(sql: String, parameters: &[Value<'_>]) -> crate::Result<String> {
    const MATCH: &str = "MATCH";
    const AGAINST: &str = "AGAINST";
    const BOOLEAN_MODE: &str = "IN BOOLEAN MODE";

    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((match_start, match_open)) = find_function(&sql, search_start, MATCH) {
        if sql.as_bytes().get(match_open) != Some(&b'(') {
            search_start = match_start + MATCH.len();
            continue;
        }

        let Some(columns_end) = matching_parenthesis(&sql, match_open) else {
            break;
        };

        let against_start = skip_whitespace(&sql, columns_end + 1);
        if !sql[against_start..].starts_with(AGAINST) {
            search_start = columns_end + 1;
            continue;
        }

        let against_open = skip_whitespace(&sql, against_start + AGAINST.len());
        if sql.as_bytes().get(against_open) != Some(&b'(') {
            search_start = against_start + AGAINST.len();
            continue;
        }

        let Some(against_end) = matching_parenthesis(&sql, against_open) else {
            break;
        };

        let Some(mode_start) = sql[against_open + 1..against_end]
            .rfind(BOOLEAN_MODE)
            .map(|offset| against_open + 1 + offset)
        else {
            search_start = against_end + 1;
            continue;
        };

        if !sql[mode_start + BOOLEAN_MODE.len()..against_end].trim().is_empty() {
            search_start = against_end + 1;
            continue;
        }

        let document = fulltext_document(&sql[match_open + 1..columns_end]);
        let query = sql[against_open + 1..mode_start].trim();
        let query_parameter_index = fulltext_query_parameter_index(query, &sql)?;
        // Validate the parameter shape without interpreting its search language.
        // Concrete and Query Compiler parameters both reach to_tsquery unchanged.
        fulltext_query_parameter(parameters, query_parameter_index)?;
        let tsquery_function = "to_tsquery";

        output.push_str(&sql[cursor..match_start]);

        if is_fulltext_relevance_expression(&sql, match_start, against_end) {
            output.push_str("ts_rank(to_tsvector('simple', ");
            output.push_str(&document);
            output.push_str("), ");
            output.push_str(tsquery_function);
            output.push_str("('simple', ");
            output.push_str(query);
            output.push_str("))");
        } else {
            output.push_str("to_tsvector('simple', ");
            output.push_str(&document);
            output.push_str(") @@ ");
            output.push_str(tsquery_function);
            output.push_str("('simple', ");
            output.push_str(query);
            output.push(')');
        }

        cursor = against_end + 1;
        search_start = cursor;
    }

    output.push_str(&sql[cursor..]);
    Ok(output)
}

fn fulltext_query_parameter_index(query: &str, sql: &str) -> crate::Result<usize> {
    if query != "?" {
        return Err(fulltext_conversion_error(
            "Kingbase MySQL full-text search requires a parameterized search string",
        ));
    }

    let query_position = query.as_ptr() as usize - sql.as_ptr() as usize;
    parameter_positions(sql)
        .binary_search(&query_position)
        .map_err(|_| fulltext_conversion_error("Kingbase MySQL full-text search parameter could not be located"))
}

fn fulltext_query_parameter<'a>(parameters: &'a [Value<'_>], index: usize) -> crate::Result<Option<&'a str>> {
    match parameters.get(index).map(|parameter| &parameter.typed) {
        Some(ValueType::Text(Some(query))) => Ok(Some(query)),
        Some(ValueType::Opaque(opaque)) if opaque.typ() == &OpaqueType::Text => Ok(None),
        _ => Err(fulltext_conversion_error(
            "Kingbase MySQL full-text search requires a non-null string search value",
        )),
    }
}

fn fulltext_conversion_error(message: impl Into<String>) -> Error {
    let message = message.into();
    let mut builder = Error::builder(ErrorKind::conversion(message.clone()));
    builder.set_original_message(message);
    builder.build()
}

fn fulltext_document(columns: &str) -> String {
    columns
        .split(',')
        .map(|column| format!("COALESCE({}, '')", column.trim()))
        .reduce(|document, column| format!("textcat({document}, textcat(' ', {column}))"))
        .expect("a MySQL MATCH expression must contain at least one column")
}

fn is_fulltext_relevance_expression(sql: &str, match_start: usize, against_end: usize) -> bool {
    let before_match = &sql[..match_start];
    let last_order_by = before_match.rfind("ORDER BY ");
    let last_where = before_match.rfind(" WHERE ");

    if last_order_by.is_some_and(|order_by| last_where.is_none_or(|where_clause| order_by > where_clause)) {
        return true;
    }

    // `text_search_relevance()` may be selected directly, before the first
    // FROM clause, rather than only used as an ORDER BY expression.
    if before_match.rfind(" FROM ").is_none() {
        return true;
    }

    // A relevance expression can also be compared in a WHERE clause. A plain
    // MATCH expression there is the Boolean filter form instead.
    matches!(
        sql[against_end + 1..].trim_start().as_bytes().first(),
        Some(b'>' | b'<' | b'=')
    )
}

fn template_sql(fragments: Vec<Fragment>) -> (String, Vec<Fragment>) {
    let mut sql = String::new();
    let mut parameter_fragments = Vec::new();

    for fragment in fragments {
        match fragment {
            Fragment::StringChunk { chunk } => sql.push_str(&chunk),
            fragment => {
                sql.push('?');
                parameter_fragments.push(fragment);
            }
        }
    }

    (sql, parameter_fragments)
}

fn rebuild_template<'a>(
    sql: String,
    parameter_fragments: Vec<Fragment>,
    parameters: Vec<Value<'a>>,
    placeholder_format: PlaceholderFormat,
) -> crate::Result<QueryTemplate<Value<'a>>> {
    let positions = parameter_positions(&sql);

    if positions.len() != parameter_fragments.len() {
        let message = format!(
            "Kingbase MySQL SQL rewrite changed the parameter count from {} to {}",
            parameter_fragments.len(),
            positions.len()
        );
        let mut builder = Error::builder(ErrorKind::conversion(message.clone()));
        builder.set_original_message(message);
        return Err(builder.build());
    }

    let mut fragments = Vec::with_capacity(parameter_fragments.len() * 2 + 1);
    let mut cursor = 0;

    for (position, fragment) in positions.into_iter().zip(parameter_fragments) {
        if cursor < position {
            fragments.push(Fragment::StringChunk {
                chunk: sql[cursor..position].to_owned(),
            });
        }
        fragments.push(fragment);
        cursor = position + 1;
    }

    if cursor < sql.len() {
        fragments.push(Fragment::StringChunk {
            chunk: sql[cursor..].to_owned(),
        });
    }

    Ok(QueryTemplate {
        fragments,
        parameters,
        placeholder_format,
    })
}

fn cast_sum_parameters(sql: String, params: &[Value<'_>]) -> String {
    let parameter_positions = parameter_positions(&sql);
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((_, open_paren)) = find_function(&sql, search_start, "SUM") {
        let Some(function_end) = matching_parenthesis(&sql, open_paren) else {
            break;
        };
        let argument_start = skip_whitespace(&sql, open_paren + 1);
        let argument_end = previous_non_whitespace(sql.as_bytes(), function_end);

        let Some(argument_end) = argument_end else {
            search_start = function_end + 1;
            continue;
        };

        if argument_start != argument_end || sql.as_bytes().get(argument_start) != Some(&b'?') {
            search_start = function_end + 1;
            continue;
        }

        let Ok(parameter_index) = parameter_positions.binary_search(&argument_start) else {
            search_start = function_end + 1;
            continue;
        };
        let Some(param) = params.get(parameter_index) else {
            search_start = function_end + 1;
            continue;
        };
        let Some(type_name) = sum_parameter_type(param) else {
            search_start = function_end + 1;
            continue;
        };

        output.push_str(&sql[cursor..argument_start]);
        output.push_str("CAST(? AS ");
        output.push_str(type_name);
        output.push(')');
        cursor = argument_end + 1;
        search_start = function_end + 1;
    }

    output.push_str(&sql[cursor..]);
    output
}

fn sum_parameter_type(param: &Value<'_>) -> Option<&'static str> {
    match param.typed {
        // The ordinary query path leaves wire parameter types unspecified.
        // Disambiguate SUM using the actual value type; the server chooses the
        // aggregate's return type. Do not promote integers to MySQL DECIMAL.
        ValueType::Int32(_) => Some("INTEGER"),
        ValueType::Int64(_) => Some("BIGINT"),
        ValueType::Float(_) => Some("FLOAT4"),
        ValueType::Double(_) => Some("FLOAT8"),
        // C010's unqualified NUMERIC defaults to (10,0). Kingbase's
        // documented FIXED type without modifiers preserves decimal scale.
        ValueType::Numeric(_) => Some("FIXED"),
        _ => None,
    }
}

fn promote_json_numeric_comparison_params(sql: &str, params: &mut [Value<'_>]) {
    for index in json_numeric_comparison_parameter_indexes(sql) {
        let Some(param) = params.get_mut(index) else {
            continue;
        };

        let json = match &param.typed {
            ValueType::Int32(Some(value)) => Some(serde_json::Value::from(*value)),
            ValueType::Int64(Some(value)) => Some(serde_json::Value::from(*value)),
            ValueType::Float(Some(value)) => serde_json::Number::from_f64(f64::from(*value)).map(Into::into),
            ValueType::Double(Some(value)) => serde_json::Number::from_f64(*value).map(Into::into),
            ValueType::Text(Some(value)) => Some(serde_json::Value::String(value.to_string())),
            _ => None,
        };

        if let Some(json) = json {
            param.typed = ValueType::Json(Some(json));
        }
    }
}

fn cast_json_comparison_expressions(sql: String) -> String {
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((function_start, open_paren)) = find_function(&sql, search_start, "JSON_EXTRACT") {
        let Some(function_end) = matching_parenthesis(&sql, open_paren) else {
            break;
        };

        let is_comparison_operand = numeric_comparison_parameter_after(&sql, function_end + 1).is_some()
            || numeric_comparison_parameter_before(&sql, function_start).is_some();

        if is_comparison_operand {
            output.push_str(&sql[cursor..function_start]);
            output.push_str("CAST(");
            output.push_str(&sql[function_start..=function_end]);
            output.push_str(" AS jsonb)");
            cursor = function_end + 1;
        }

        search_start = function_end + 1;
    }

    output.push_str(&sql[cursor..]);
    output
}

fn json_numeric_comparison_parameter_indexes(sql: &str) -> Vec<usize> {
    let parameter_positions = parameter_positions(sql);
    let mut indexes = Vec::new();
    let mut search_start = 0;

    while let Some((function_start, open_paren)) = find_function(sql, search_start, "JSON_EXTRACT") {
        let Some(function_end) = matching_parenthesis(sql, open_paren) else {
            break;
        };

        if let Some(parameter_position) = numeric_comparison_parameter_after(sql, function_end + 1)
            && let Ok(index) = parameter_positions.binary_search(&parameter_position)
        {
            indexes.push(index);
        }

        if let Some(parameter_position) = numeric_comparison_parameter_before(sql, function_start)
            && let Ok(index) = parameter_positions.binary_search(&parameter_position)
        {
            indexes.push(index);
        }

        search_start = function_end + 1;
    }

    indexes.sort_unstable();
    indexes.dedup();
    indexes
}

fn parameter_positions(sql: &str) -> Vec<usize> {
    let bytes = sql.as_bytes();
    let mut positions = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(end) = non_code_end(sql, index) {
            index = end;
            continue;
        }

        if byte == b'?' {
            positions.push(index);
        }

        index += 1;
    }

    positions
}

fn numeric_comparison_parameter_after(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut index = skip_whitespace(sql, start);

    match bytes.get(index)? {
        b'>' | b'<' => {
            index += 1;
            if bytes.get(index) == Some(&b'=') {
                index += 1;
            }

            let parameter = skip_whitespace(sql, index);
            (bytes.get(parameter) == Some(&b'?')).then_some(parameter)
        }
        _ => None,
    }
}

fn numeric_comparison_parameter_before(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let operator_end = previous_non_whitespace(bytes, start)?;
    let parameter_boundary = match bytes[operator_end] {
        b'>' | b'<' => operator_end,
        b'=' if previous_non_whitespace(bytes, operator_end)
            .is_some_and(|index| matches!(bytes[index], b'>' | b'<')) =>
        {
            previous_non_whitespace(bytes, operator_end)?
        }
        _ => return None,
    };

    let parameter = previous_non_whitespace(bytes, parameter_boundary)?;
    (bytes[parameter] == b'?').then_some(parameter)
}

fn previous_non_whitespace(bytes: &[u8], start: usize) -> Option<usize> {
    (0..start).rev().find(|index| !bytes[*index].is_ascii_whitespace())
}

fn json_extract_path_parameter_indexes(sql: &str) -> Vec<usize> {
    #[derive(Clone, Copy)]
    struct Context {
        is_json_extract: bool,
        argument: usize,
    }

    let bytes = sql.as_bytes();
    let mut contexts = Vec::new();
    let mut indexes = Vec::new();
    let mut parameter_index = 0;
    let mut pending_json_extract = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(end) = non_code_end(sql, index) {
            index = end;
            continue;
        }

        match byte {
            b'(' => {
                contexts.push(Context {
                    is_json_extract: pending_json_extract,
                    argument: 0,
                });
                pending_json_extract = false;
            }
            b')' => {
                contexts.pop();
                pending_json_extract = false;
            }
            b',' => {
                if let Some(context) = contexts.last_mut()
                    && context.is_json_extract
                {
                    context.argument += 1;
                }
                pending_json_extract = false;
            }
            b'?' => {
                if contexts
                    .last()
                    .is_some_and(|context| context.is_json_extract && context.argument == 1)
                {
                    indexes.push(parameter_index);
                }
                parameter_index += 1;
                pending_json_extract = false;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let identifier_start = index;
                index += 1;

                while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
                    index += 1;
                }

                pending_json_extract = sql[identifier_start..index].eq_ignore_ascii_case("JSON_EXTRACT");
                continue;
            }
            byte if byte.is_ascii_whitespace() => {}
            _ => pending_json_extract = false,
        }

        index += 1;
    }

    indexes
}

fn cast_dynamic_jsonpaths(sql: String) -> String {
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((_, open_paren)) = find_function(&sql, search_start, "JSON_EXTRACT") {
        let Some(function_end) = matching_parenthesis(&sql, open_paren) else {
            break;
        };
        let Some(comma) = first_function_argument_comma(&sql, open_paren, function_end) else {
            search_start = function_end + 1;
            continue;
        };

        let path_start = skip_whitespace(&sql, comma + 1);
        if !sql[path_start..].starts_with("CONCAT(") {
            search_start = function_end + 1;
            continue;
        }

        let path_open_paren = path_start + "CONCAT".len();
        let Some(path_end) = matching_parenthesis(&sql, path_open_paren).map(|end| end + 1) else {
            search_start = function_end + 1;
            continue;
        };

        if path_end > function_end || !sql[path_end..function_end].trim().is_empty() {
            search_start = function_end + 1;
            continue;
        }

        output.push_str(&sql[cursor..path_start]);
        output.push_str("CAST(");
        output.push_str(&sql[path_start..path_end]);
        output.push_str(" AS JSONPATH)");
        cursor = path_end;
        search_start = function_end + 1;
    }

    output.push_str(&sql[cursor..]);
    output
}

fn qualify_json_contains(sql: String) -> String {
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((function_start, open_paren)) = find_function(&sql, search_start, "JSON_CONTAINS") {
        let Some(function_end) = matching_parenthesis(&sql, open_paren) else {
            break;
        };

        if function_argument_count(&sql, open_paren, function_end) == 2 {
            output.push_str(&sql[cursor..function_start]);
            output.push_str("sys.JSON_CONTAINS");
            output.push_str(&sql[open_paren..function_end]);
            output.push_str(", CAST('$' AS JSONPATH)");
            cursor = function_end;
        }

        search_start = function_end + 1;
    }

    output.push_str(&sql[cursor..]);
    output
}

fn unquote_json_values(sql: String) -> String {
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((function_start, open_paren)) = find_function(&sql, search_start, "JSON_UNQUOTE") {
        let Some(function_end) = matching_parenthesis(&sql, open_paren) else {
            break;
        };

        output.push_str(&sql[cursor..function_start]);
        output.push_str("(CAST(");
        output.push_str(&sql[open_paren + 1..function_end]);
        output.push_str(" AS jsonb) ->> CAST('$' AS JSONPATH))");
        cursor = function_end + 1;
        search_start = function_end + 1;
    }

    output.push_str(&sql[cursor..]);
    output
}

fn native_uuid_as_text(sql: String) -> String {
    let mut output = String::with_capacity(sql.len());
    let mut cursor = 0;
    let mut search_start = 0;

    while let Some((start, open)) = find_function(&sql, search_start, "uuid") {
        let Some(end) = matching_parenthesis(&sql, open) else {
            break;
        };
        if sql[open + 1..end].trim().is_empty() {
            output.push_str(&sql[cursor..start]);
            output.push_str("CAST(sys_guid() AS text)");
            cursor = end + 1;
        }
        search_start = end + 1;
    }
    output.push_str(&sql[cursor..]);
    output
}

/// Find an unqualified function token in SQL code, never inside a quoted
/// identifier, literal or comment. Callers resume after a complete function.
fn find_function(sql: &str, start: usize, name: &str) -> Option<(usize, usize)> {
    let bytes = sql.as_bytes();
    let mut index = start;
    while index < bytes.len() {
        if let Some(end) = non_code_end(sql, index) {
            index = end;
        } else if is_identifier_byte(bytes[index]) {
            let token_start = index;
            while index < bytes.len() && is_identifier_byte(bytes[index]) {
                index += 1;
            }
            if sql[token_start..index].eq_ignore_ascii_case(name)
                && !previous_non_whitespace(bytes, token_start).is_some_and(|pos| bytes[pos] == b'.')
            {
                let open = skip_whitespace(sql, index);
                if bytes.get(open) == Some(&b'(') {
                    return Some((token_start, open));
                }
            }
        } else {
            index += 1;
        }
    }
    None
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$') || !byte.is_ascii()
}

/// Return the first byte after a quoted/commented region. Handle both MySQL
/// quoting and the dollar strings accepted by Kingbase MySQL mode.
fn non_code_end(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let first = *bytes.get(start)?;
    if matches!(first, b'\'' | b'"' | b'`') {
        let mut index = start + 1;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index = (index + 2).min(bytes.len());
            } else if bytes[index] == first {
                if bytes.get(index + 1) == Some(&first) {
                    index += 2;
                } else {
                    return Some(index + 1);
                }
            } else {
                index += 1;
            }
        }
        return Some(bytes.len());
    }
    if first == b'#' || bytes[start..].starts_with(b"--") {
        return Some(sql[start..].find('\n').map_or(bytes.len(), |offset| start + offset + 1));
    }
    if bytes[start..].starts_with(b"/*") {
        let mut index = start + 2;
        let mut depth = 1;
        while index + 1 < bytes.len() {
            if bytes[index..].starts_with(b"/*") {
                depth += 1;
                index += 2;
            } else if bytes[index..].starts_with(b"*/") {
                depth -= 1;
                index += 2;
                if depth == 0 {
                    return Some(index);
                }
            } else {
                index += 1;
            }
        }
        return Some(bytes.len());
    }
    if first == b'$' {
        let mut end = start + 1;
        if bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            while bytes
                .get(end)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                end += 1;
            }
        }
        if bytes.get(end) == Some(&b'$') {
            let tag = &sql[start..=end];
            return Some(
                sql[end + 1..]
                    .find(tag)
                    .map_or(bytes.len(), |offset| end + 1 + offset + tag.len()),
            );
        }
    }
    None
}

fn skip_whitespace(sql: &str, mut index: usize) -> usize {
    while sql.as_bytes().get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }

    index
}

fn first_function_argument_comma(sql: &str, open_paren: usize, function_end: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut depth = 1;
    let mut index = open_paren + 1;

    while index < function_end {
        let byte = bytes[index];

        if let Some(end) = non_code_end(sql, index) {
            index = end;
            continue;
        }

        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 1 => return Some(index),
            _ => {}
        }

        index += 1;
    }

    None
}

fn function_argument_count(sql: &str, open_paren: usize, function_end: usize) -> usize {
    let bytes = sql.as_bytes();
    let mut count = 1;
    let mut depth = 1;
    let mut index = open_paren + 1;

    if sql[index..function_end].trim().is_empty() {
        return 0;
    }

    while index < function_end {
        let byte = bytes[index];

        if let Some(end) = non_code_end(sql, index) {
            index = end;
            continue;
        }

        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 1 => count += 1,
            _ => {}
        }

        index += 1;
    }

    count
}

fn matching_parenthesis(sql: &str, open_paren: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut depth = 0;
    let mut index = open_paren;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(end) = non_code_end(sql, index) {
            index = end;
            continue;
        }

        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }

        index += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::KingbaseMysql;
    use crate::Value;
    use crate::ast::{
        Aliasable, Column, Comparable, Expression, JsonPath, OpaqueType, Select, Table, ValueType, json_extract,
        json_unquote, native_uuid, text_search, text_search_relevance,
    };
    use crate::visitor::{Mysql, Visitor};

    #[test]
    fn identifier_escaping_is_scoped_to_kingbase_without_changing_values() {
        let value = "a`b 'literal'";
        let query = Select::from_table(Table::from("ta`ble").alias("al`ias"))
            .column(Column::from(("al`ias", "co`l")).alias("out`put"))
            .so_that(Column::from(("al`ias", "co`l")).equals(value));

        let (kingbase, parameters) = KingbaseMysql::build(query.clone()).unwrap();
        assert_eq!(
            kingbase,
            "SELECT `al``ias`.`co``l` AS `out``put` FROM `ta``ble` AS `al``ias` WHERE `al``ias`.`co``l` = ?"
        );
        assert_eq!(parameters, vec![Value::text(value)]);

        // Preserve the upstream MySQL visitor's output even for embedded quote
        // characters: Kingbase fixes must not silently change native MySQL.
        let (mysql, native_parameters) = Mysql::build(query).unwrap();
        assert_eq!(
            mysql,
            "SELECT `al`ias`.`co`l` AS `out`put` FROM `ta`ble` AS `al`ias` WHERE `al`ias`.`co`l` = ?"
        );
        assert_eq!(native_parameters, parameters);
    }

    #[test]
    fn sum_parameters_keep_their_original_types() {
        for (parameter, sql) in [
            (Value::int32(1), "SELECT SUM(CAST(? AS INTEGER))"),
            (Value::int64(1), "SELECT SUM(CAST(? AS BIGINT))"),
            (Value::float(1.5), "SELECT SUM(CAST(? AS FLOAT4))"),
            (Value::double(1.5), "SELECT SUM(CAST(? AS FLOAT8))"),
            (Value::numeric("1.5".parse().unwrap()), "SELECT SUM(CAST(? AS FIXED))"),
            (Value::from(None::<i32>), "SELECT SUM(CAST(? AS INTEGER))"),
        ] {
            let (actual, params) =
                KingbaseMysql::build(Select::default().value(crate::ast::sum(parameter.clone()))).unwrap();
            assert_eq!(actual, sql);
            assert_eq!(params, vec![parameter]);
        }
    }

    #[test]
    fn function_shaped_identifiers_are_preserved() {
        for name in [
            "uuid()",
            "JSON_UNQUOTE(v)",
            "JSON_CONTAINS(a,b)",
            "JSON_EXTRACT(v,CONCAT(a,b))",
            "SUM(?)",
            "MATCH(a) AGAINST (? IN BOOLEAN MODE)",
            "中文uuid()",
        ] {
            let (sql, params) = KingbaseMysql::build(Select::default().column(Column::from(name))).unwrap();
            assert_eq!(sql, format!("SELECT `{name}`"));
            assert!(params.is_empty());
        }
    }

    #[test]
    fn uuid_rewrite_skips_quoted_regions_comments_and_other_identifiers() {
        let protected = r#"`uuid()` 'uuid()' "uuid()" `a``uuid()` 'a''uuid()'
            $$uuid()$$ $tag$uuid()$tag$ myuuid() sys.uuid() 中文uuid()
            /* uuid() /* uuid() */ uuid() */ -- uuid()
            # uuid()
        "#;
        let sql = format!("SELECT {protected}, UUID (), uuid()");
        assert_eq!(
            super::native_uuid_as_text(sql),
            format!("SELECT {protected}, CAST(sys_guid() AS text), CAST(sys_guid() AS text)")
        );
    }

    #[test]
    fn json_rewrite_keeps_function_shaped_names_and_bind_positions() {
        let query = Select::from_table("uuid()")
            .column(Column::from("JSON_CONTAINS(a,b)"))
            .value(json_unquote(Column::from("JSON_UNQUOTE(v)")))
            .value(native_uuid());
        let (sql, params) = KingbaseMysql::build(query).unwrap();
        assert_eq!(
            sql,
            "SELECT `JSON_CONTAINS(a,b)`, (CAST(`JSON_UNQUOTE(v)` AS jsonb) ->> CAST('$' AS JSONPATH)), \
             CAST(sys_guid() AS text) FROM `uuid()`"
        );
        assert!(params.is_empty());

        let sql = "SELECT `JSON_EXTRACT(?)`, 'SUM(?)', \"uuid(?)\", $$?$$, /* ? */ ? -- ?\n, ?";
        let positions = super::parameter_positions(sql);
        assert_eq!(positions.len(), 2);
        assert_eq!(&sql[positions[0]..positions[0] + 1], "?");
        assert_eq!(positions[1], sql.len() - 1);
    }

    #[test]
    fn json_unquote_uses_the_unambiguous_jsonpath_operator() {
        let query = Select::default().value(json_unquote(col!("json")));
        let (sql, params) = KingbaseMysql::build(query).unwrap();

        assert_eq!("SELECT (CAST(`json` AS jsonb) ->> CAST('$' AS JSONPATH))", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn native_uuid_is_a_text_value() {
        let (sql, params) = KingbaseMysql::build(Select::default().value(native_uuid())).unwrap();

        assert_eq!("SELECT CAST(sys_guid() AS text)", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn build_template_preserves_jsonpath_parameter_metadata() {
        let query = Select::default().value(json_extract(Column::from("json"), JsonPath::string("$.a"), false));
        let template = KingbaseMysql::build_template(query).unwrap();

        assert_eq!("SELECT JSON_EXTRACT(`json`, ?)", template.to_sql().unwrap());
        assert_eq!(template.parameters.len(), 1);
        assert!(matches!(
            template.parameters[0]
                .native_column_type
                .as_ref()
                .map(|t| t.name.as_ref()),
            Some("JSONPATH")
        ));
        assert!(matches!(template.parameters[0].typed, ValueType::Text(Some(_))));
    }

    #[test]
    fn full_text_filters_use_kingbase_syntax() {
        let search: Expression = text_search(&[Column::from("name"), Column::from("email")]).into();
        let query = Select::from_table("User").so_that(search.matches("John & Smith"));
        let (sql, params) = KingbaseMysql::build(query).unwrap();

        assert_eq!(
            "SELECT `User`.* FROM `User` WHERE to_tsvector('simple', textcat(COALESCE(`name`, ''), textcat(' ', COALESCE(`email`, '')))) @@ to_tsquery('simple', ?)",
            sql
        );
        assert_eq!(params, vec![Value::text("John & Smith")]);
    }

    #[test]
    fn full_text_filters_preserve_native_tsquery_parameters() {
        let search: Expression = text_search(&[Column::from("name")]).into();

        for query in [
            "John & Smith",
            "John | Smith",
            "John & !Smith",
            "!Smith",
            "John:*",
            "John <-> Smith",
            "John <2> Smith",
            "(John | April) & !Smith",
            "'John':A",
        ] {
            let (_, params) =
                KingbaseMysql::build(Select::from_table("User").so_that(search.clone().matches(query))).unwrap();
            assert_eq!(params, vec![Value::text(query)]);
        }
    }

    #[test]
    fn full_text_filters_preserve_dynamic_string_parameters() {
        let search: Expression = text_search(&[Column::from("name")]).into();
        let query =
            Select::from_table("User").so_that(search.matches(Value::opaque("search".to_owned(), OpaqueType::Text)));
        let (sql, params) = KingbaseMysql::build(query).unwrap();

        assert_eq!(
            "SELECT `User`.* FROM `User` WHERE to_tsvector('simple', COALESCE(`name`, '')) @@ to_tsquery('simple', ?)",
            sql
        );
        assert!(matches!(
            params[0].typed,
            ValueType::Opaque(ref opaque) if opaque.typ() == &OpaqueType::Text
        ));
        assert!(params[0].native_column_type.is_none());
    }

    #[test]
    fn full_text_query_syntax_is_validated_by_the_database() {
        let search: Expression = text_search(&[Column::from("name")]).into();
        for query in ["John Smith", "+John +Smith", "(John", ""] {
            let (_, params) =
                KingbaseMysql::build(Select::from_table("User").so_that(search.clone().matches(query))).unwrap();
            assert_eq!(params, vec![Value::text(query)]);
        }
    }

    #[test]
    fn full_text_relevance_uses_kingbase_syntax() {
        let relevance: Expression = text_search_relevance(&[Column::from("name")], "John & Smith").into();
        let (sql, params) = KingbaseMysql::build(Select::from_table("User").value(relevance)).unwrap();

        assert_eq!(
            "SELECT ts_rank(to_tsvector('simple', COALESCE(`name`, '')), to_tsquery('simple', ?)) FROM `User`",
            sql
        );
        assert_eq!(params, vec![Value::text("John & Smith")]);
    }
}
