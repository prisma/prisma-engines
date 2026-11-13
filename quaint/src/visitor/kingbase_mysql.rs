use crate::{
    ast::{Expression, ExpressionKind, OpaqueType, Query, TypeFamily, Value, ValueType},
    error::{Error, ErrorKind},
    visitor::{self, Mysql, Visitor, mysql::MysqlVisitorHooks},
};
use query_template::{Fragment, PlaceholderFormat, QueryTemplate};

/// A MySQL-compatible SQL visitor for KingbaseES.
///
/// It reuses MySQL SQL generation with local type conversion and full-text adaptations.
pub struct KingbaseMysql;

impl KingbaseMysql {
    pub fn build_template<'a, Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let template = Mysql::build_template_with_hooks(
            query,
            true,
            MysqlVisitorHooks {
                json_build_obj_expr: Some(visit_json_build_obj_expr),
                raw_json: Some(visit_raw_json),
            },
        )?;
        let QueryTemplate {
            fragments,
            parameters,
            placeholder_format,
        } = template;
        let (sql, parameter_fragments) = template_sql(fragments);

        let sql = rewrite_full_text_search(sql, &parameters)?;

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

/// Render native values inside JSON using the casts accepted by KES MySQL.
fn visit_json_build_obj_expr<'a>(visitor: &mut Mysql<'a>, expr: Expression<'a>) -> visitor::Result {
    if let ExpressionKind::Column(column) = expr.kind() {
        match (column.type_family.as_ref(), column.native_type.as_deref()) {
            (
                Some(TypeFamily::Text(_)),
                Some("LONGBLOB" | "BLOB" | "MEDIUMBLOB" | "SMALLBLOB" | "TINYBLOB" | "VARBINARY" | "BINARY" | "BIT"),
            ) => {
                return visitor.surround_with("to_base64(CAST(", " AS VARBINARY))", |visitor| {
                    visitor.visit_expression(expr)
                });
            }
            // Keep values as strings in JSON to preserve numeric precision.
            (_, Some("FLOAT")) | (Some(TypeFamily::Int), Some("BIGINT" | "UNSIGNEDBIGINT")) => {
                return visitor.surround_with("CAST(", " AS CHAR)", |visitor| visitor.visit_expression(expr));
            }
            _ => {}
        }
    }

    visitor.visit_expression(expr)
}

fn visit_raw_json(visitor: &mut Mysql<'_>, value: &serde_json::Value) -> visitor::Result {
    let json = serde_json::to_string(value)?.replace('\'', "''");
    visitor.write(format!("CAST('{json}' AS JSON)"))
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
        validate_fulltext_query_parameter(parameters, query_parameter_index)?;

        output.push_str(&sql[cursor..match_start]);

        if is_fulltext_relevance_expression(&sql, match_start, against_end) {
            output.push_str("ts_rank(to_tsvector('simple', ");
            output.push_str(&document);
            output.push_str("), to_tsquery('simple', ");
            output.push_str(query);
            output.push_str("))");
        } else {
            output.push_str("to_tsvector('simple', ");
            output.push_str(&document);
            output.push_str(") @@ to_tsquery('simple', ");
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

fn validate_fulltext_query_parameter(parameters: &[Value<'_>], index: usize) -> crate::Result<()> {
    match parameters.get(index).map(|parameter| &parameter.typed) {
        Some(ValueType::Text(Some(_))) => Ok(()),
        Some(ValueType::Opaque(opaque)) if opaque.typ() == &OpaqueType::Text => Ok(()),
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

fn previous_non_whitespace(bytes: &[u8], start: usize) -> Option<usize> {
    (0..start).rev().find(|index| !bytes[*index].is_ascii_whitespace())
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
        Aliasable, Column, Comparable, Expression, IntoRaw, JsonPath, OpaqueType, Select, Table, ValueType,
        json_extract, json_unquote, native_uuid, text_search, text_search_relevance,
    };
    use crate::visitor::{Mysql, Visitor};

    #[test]
    fn raw_json_literals_use_cast_and_preserve_quotes() {
        for value in [serde_json::json!([]), serde_json::json!({"text": "O'Reilly"})] {
            let literal = serde_json::to_string(&value).unwrap();
            let query = Select::default().value(Value::json(value).raw());
            let (sql, params) = KingbaseMysql::build(query.clone()).unwrap();
            assert_eq!(sql, format!("SELECT CAST('{}' AS JSON)", literal.replace('\'', "''")));
            assert!(params.is_empty());
            let (sql, params) = Mysql::build(query).unwrap();
            assert_eq!(sql, format!("SELECT CONVERT('{literal}', JSON)"));
            assert!(params.is_empty());
        }
    }

    #[test]
    fn json_build_object_casts_binary_types_without_losing_bit_width() {
        for native_type in [
            "BIT",
            "BINARY",
            "VARBINARY",
            "TINYBLOB",
            "SMALLBLOB",
            "MEDIUMBLOB",
            "BLOB",
            "LONGBLOB",
        ] {
            let column = Column::from("value")
                .native_column_type(Some(native_type))
                .type_family(crate::ast::TypeFamily::Text(None));
            let query = Select::default().value(crate::ast::json_build_object(vec![("value".into(), column.into())]));
            let (sql, params) = KingbaseMysql::build(query.clone()).unwrap();
            assert_eq!(
                sql,
                "SELECT JSON_OBJECT('value', to_base64(CAST(`value` AS VARBINARY)))"
            );
            assert!(params.is_empty());
            let (sql, params) = Mysql::build(query).unwrap();
            assert_eq!(sql, "SELECT JSON_OBJECT('value', to_base64(`value`))");
            assert!(params.is_empty());
        }
    }

    #[test]
    fn json_build_object_casts_float_and_bigint_with_native_syntax() {
        for (native_type, family) in [
            ("FLOAT", crate::ast::TypeFamily::Float),
            ("BIGINT", crate::ast::TypeFamily::Int),
            ("UNSIGNEDBIGINT", crate::ast::TypeFamily::Int),
        ] {
            let column = Column::from("value")
                .native_column_type(Some(native_type))
                .type_family(family);
            let query = Select::default().value(crate::ast::json_build_object(vec![("value".into(), column.into())]));
            let (sql, params) = KingbaseMysql::build(query.clone()).unwrap();
            assert_eq!(sql, "SELECT JSON_OBJECT('value', CAST(`value` AS CHAR))");
            assert!(params.is_empty());
            let (sql, params) = Mysql::build(query).unwrap();
            assert_eq!(sql, "SELECT JSON_OBJECT('value', CONVERT(`value`, CHAR))");
            assert!(params.is_empty());
        }
    }

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
    fn sum_parameters_use_mysql_sql_and_keep_their_types() {
        for parameter in [
            Value::int32(1),
            Value::int64(2147483648_i64),
            Value::float(0.1),
            Value::double(0.1),
            Value::numeric("1.5".parse().unwrap()),
            Value::null_int32(),
            Value::null_int64(),
            Value::null_float(),
            Value::null_double(),
            Value::null_numeric(),
        ] {
            let query = Select::default().value(crate::ast::sum(parameter.clone()));
            let (actual, params) = KingbaseMysql::build(query.clone()).unwrap();
            let (mysql, mysql_params) = Mysql::build(query).unwrap();
            assert_eq!(actual, "SELECT SUM(?)");
            assert_eq!(actual, mysql);
            assert_eq!(params, mysql_params);
            assert_eq!(params, vec![parameter]);
        }
    }

    #[test]
    fn decimal_sum_matches_mysql_without_coercing_parameters_or_columns() {
        for parameter in [
            Value::numeric("1.25".parse().unwrap()),
            Value::numeric("-1.25".parse().unwrap()),
            Value::numeric("1E-31".parse().unwrap()),
            Value::numeric("123456789012345678901234567890.123456789".parse().unwrap()),
            Value::null_numeric(),
        ] {
            let query = Select::from_table("Product")
                .value(crate::ast::sum(parameter.clone()))
                .value(crate::ast::sum(Column::from("price")));
            let (actual, parameters) = KingbaseMysql::build(query.clone()).unwrap();
            let (mysql, mysql_parameters) = Mysql::build(query).unwrap();

            assert_eq!(actual, mysql);
            assert_eq!(parameters, mysql_parameters);
            assert_eq!(parameters, vec![parameter]);
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
    fn json_rewrite_keeps_function_shaped_names_and_bind_positions() {
        let query = Select::from_table("uuid()")
            .column(Column::from("JSON_CONTAINS(a,b)"))
            .value(json_unquote(Column::from("JSON_UNQUOTE(v)")))
            .value(native_uuid());
        let (sql, params) = KingbaseMysql::build(query).unwrap();
        assert_eq!(
            sql,
            "SELECT `JSON_CONTAINS(a,b)`, JSON_UNQUOTE(`JSON_UNQUOTE(v)`), \
             uuid() FROM `uuid()`"
        );
        assert!(params.is_empty());

        let sql = "SELECT `JSON_EXTRACT(?)`, 'SUM(?)', \"uuid(?)\", $$?$$, /* ? */ ? -- ?\n, ?";
        let positions = super::parameter_positions(sql);
        assert_eq!(positions.len(), 2);
        assert_eq!(&sql[positions[0]..positions[0] + 1], "?");
        assert_eq!(positions[1], sql.len() - 1);
    }

    #[test]
    fn json_unquote_uses_mysql_for_columns_parameters_and_nested_calls() {
        for query in [
            Select::default().value(json_unquote(col!("json"))),
            Select::default().value(json_unquote(Value::text("hello"))),
            Select::default().value(json_unquote(Value::json(serde_json::json!({"a": 1})))),
            Select::default().value(json_unquote(Value::null_text())),
            Select::default().value(json_unquote(json_unquote(Value::text("\"hello\"")))),
        ] {
            let (sql, params) = KingbaseMysql::build(query.clone()).unwrap();
            let (mysql, mysql_params) = Mysql::build(query).unwrap();
            assert_eq!(sql, mysql);
            assert_eq!(params, mysql_params);
            assert!(sql.contains("JSON_UNQUOTE("), "{sql}");
            assert!(!sql.contains(" AS jsonb") && !sql.contains("JSONPATH"), "{sql}");
        }
    }

    #[test]
    fn json_unquote_keeps_mysql_path_parameters_and_bind_order() {
        let path = "$.name";
        let query = Select::from_table("data")
            .value(Value::text("before"))
            .value(json_unquote(json_extract(col!("json"), JsonPath::string(path), false)))
            .value(Value::int32(42));
        let mysql = Mysql::build_template(query.clone()).unwrap();
        let template = KingbaseMysql::build_template(query).unwrap();
        assert_eq!(
            template.to_sql().unwrap(),
            "SELECT ?, JSON_UNQUOTE(JSON_EXTRACT(`json`, ?)), ? FROM `data`"
        );
        assert_eq!(template.parameters, mysql.parameters);
        assert_eq!(
            template.parameters,
            vec![Value::text("before"), Value::text(path), Value::int32(42),]
        );
    }

    #[test]
    fn native_uuid_uses_mysql_function() {
        let query = Select::default().value(native_uuid());
        let (mysql, mysql_params) = Mysql::build(query.clone()).unwrap();
        let (sql, params) = KingbaseMysql::build(query).unwrap();

        assert_eq!("SELECT uuid()", sql);
        assert_eq!(sql, mysql);
        assert_eq!(params, mysql_params);
        assert!(params.is_empty());
    }

    #[test]
    fn json_extract_keeps_mysql_path_parameters() {
        for path in ["$.a", "$.\"汉字\"", "$[1]", "$.\"a\\\":{\""] {
            for extract_as_string in [false, true] {
                let query = Select::default().value(json_extract(
                    Column::from("json"),
                    JsonPath::string(path),
                    extract_as_string,
                ));
                let mysql = Mysql::build_template(query.clone()).unwrap();
                let template = KingbaseMysql::build_template(query).unwrap();

                assert_eq!(template.to_sql().unwrap(), mysql.to_sql().unwrap());
                assert_eq!(template.parameters, mysql.parameters);
                assert_eq!(template.parameters, vec![Value::text(path)]);
            }
        }
    }

    #[test]
    fn json_contains_and_dynamic_paths_use_mysql_sql_and_parameters() {
        let document: Expression = col!("json").into();
        for condition in [
            document.clone().json_array_contains(serde_json::json!([1, null])),
            document.clone().json_array_not_contains(serde_json::json!(["a"])),
            document.clone().json_array_begins_with(serde_json::json!(1)),
            document.clone().json_array_not_begins_with(serde_json::json!(1)),
            document.clone().json_array_ends_into(serde_json::json!(2)),
            document.json_array_not_ends_into(serde_json::json!(2)),
        ] {
            let query = Select::from_table("data")
                .value(Value::text("before"))
                .so_that(condition);
            let (mysql, mysql_parameters) = Mysql::build(query.clone()).unwrap();
            let (sql, parameters) = KingbaseMysql::build(query).unwrap();
            assert_eq!(sql, mysql);
            assert_eq!(parameters, mysql_parameters);
            assert!(!sql.contains("JSONPATH") && !sql.contains("sys.JSON_CONTAINS"), "{sql}");
        }
    }

    #[test]
    fn json_comparisons_use_mysql_sql_and_scalar_parameters() {
        for (value, parameter) in [
            (serde_json::json!(1), Value::int64(1)),
            (serde_json::json!(1.25), Value::double(1.25)),
            (serde_json::json!("alice"), Value::text("alice")),
        ] {
            let path: Expression = json_extract(col!("json"), JsonPath::string("$.a"), false).into();
            let scalar: Expression = Value::json(value.clone()).into();
            for (condition, sign, reversed) in [
                (path.clone().greater_than(scalar.clone()), ">", false),
                (path.clone().greater_than_or_equals(scalar.clone()), ">=", false),
                (path.clone().less_than(scalar.clone()), "<", false),
                (path.clone().less_than_or_equals(scalar.clone()), "<=", false),
                (scalar.clone().greater_than(path.clone()), ">", true),
                (scalar.clone().greater_than_or_equals(path.clone()), ">=", true),
                (scalar.clone().less_than(path.clone()), "<", true),
                (scalar.less_than_or_equals(path), "<=", true),
            ] {
                let query = Select::from_table("data")
                    .value(Value::text("before"))
                    .so_that(condition);
                let (mysql, mysql_parameters) = Mysql::build(query.clone()).unwrap();
                let (sql, parameters) = KingbaseMysql::build(query).unwrap();
                assert_eq!(sql, mysql);
                assert_eq!(parameters, mysql_parameters);
                let extract = "JSON_EXTRACT(`json`, ?)";
                let (comparison, expected_parameters) = if reversed {
                    (
                        format!("? {sign} {extract}"),
                        vec![Value::text("before"), parameter.clone(), Value::text("$.a")],
                    )
                } else {
                    (
                        format!("{extract} {sign} ?"),
                        vec![Value::text("before"), Value::text("$.a"), parameter.clone()],
                    )
                };
                assert_eq!(sql, format!("SELECT ? FROM `data` WHERE {comparison}"));
                assert_eq!(parameters, expected_parameters);
            }
        }
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
