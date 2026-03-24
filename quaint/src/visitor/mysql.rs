use crate::visitor::query_writer::QueryWriter;
use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use query_template::{PlaceholderFormat, QueryTemplate};
use std::borrow::Cow;
use std::fmt;

/// A visitor to generate queries for the MySQL database.
///
/// The returned parameter values can be used directly with the mysql crate.
pub struct Mysql<'a> {
    query_template: QueryTemplate<Value<'a>>,
    /// The table a deleting or updating query is acting on.
    target_table: Option<Table<'a>>,
}

impl<'a> Mysql<'a> {
    /// Expression that evaluates to the current MySQL version.
    pub const fn version_expr() -> &'static str {
        "version()"
    }

    fn visit_regular_equality_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_expression(left)?;
        self.write(" = ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_regular_difference_comparison(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_expression(left)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_numeric_comparison(&mut self, left: Expression<'a>, right: Expression<'a>, sign: &str) -> visitor::Result {
        fn json_to_quaint_value<'a>(json: serde_json::Value) -> crate::Result<Value<'a>> {
            match json {
                serde_json::Value::String(str) => Ok(Value::text(str)),
                serde_json::Value::Number(number) => {
                    if let Some(int) = number.as_i64() {
                        // NOTE: JS numbers are 64bit numbers
                        Ok(Value::int64(int))
                    } else if let Some(float) = number.as_f64() {
                        Ok(Value::double(float))
                    } else {
                        unreachable!()
                    }
                }
                x => {
                    let msg = format!("Expected JSON string or number, found: {x}");
                    let kind = ErrorKind::conversion(msg.clone());

                    let mut builder = Error::builder(kind);
                    builder.set_original_message(msg);

                    Err(builder.build())
                }
            }
        }

        match (left, right) {
            (left, right) if left.is_extractable_json_value() && right.is_fun_retuning_json() => {
                let quaint_value = json_to_quaint_value(left.into_json_value().unwrap())?;

                self.visit_parameterized(quaint_value)?;
                self.write(format!(" {sign} "))?;
                self.visit_expression(right)?;
            }

            (left, right) if left.is_fun_retuning_json() && right.is_extractable_json_value() => {
                let quaint_value = json_to_quaint_value(right.into_json_value().unwrap())?;

                self.visit_expression(left)?;
                self.write(format!(" {sign} "))?;
                self.visit_parameterized(quaint_value)?;
            }
            (left, right) => {
                self.visit_expression(left)?;
                self.write(format!(" {sign} "))?;
                self.visit_expression(right)?;
            }
        }

        Ok(())
    }

    fn visit_order_by(&mut self, direction: &str, value: Expression<'a>) -> visitor::Result {
        self.visit_expression(value)?;
        self.write(format!(" {direction}"))?;

        Ok(())
    }

    fn visit_json_build_obj_expr(&mut self, expr: Expression<'a>) -> crate::Result<()> {
        match expr.kind() {
            // Convert bytes data to base64
            ExpressionKind::Column(col) => match (col.type_family.as_ref(), col.native_type.as_deref()) {
                (
                    Some(TypeFamily::Text(_)),
                    Some("LONGBLOB") | Some("BLOB") | Some("MEDIUMBLOB") | Some("SMALLBLOB") | Some("TINYBLOB")
                    | Some("VARBINARY") | Some("BINARY") | Some("BIT"),
                ) => {
                    self.write("to_base64")?;
                    self.surround_with("(", ")", |s| s.visit_expression(expr))?;

                    Ok(())
                }
                // Convert floats to string to avoid losing precision
                (_, Some("FLOAT")) => {
                    self.write("CONVERT")?;
                    self.surround_with("(", ")", |s| {
                        s.visit_expression(expr)?;
                        s.write(", ")?;
                        s.write("CHAR")
                    })?;
                    Ok(())
                }
                // Convert BigInt to string to preserve precision when parsed by JavaScript.
                (Some(TypeFamily::Int), Some("BIGINT" | "UNSIGNEDBIGINT")) => {
                    self.write("CONVERT")?;
                    self.surround_with("(", ")", |s| {
                        s.visit_expression(expr)?;
                        s.write(", ")?;
                        s.write("CHAR")
                    })?;
                    Ok(())
                }
                _ => self.visit_expression(expr),
            },
            _ => self.visit_expression(expr),
        }
    }
}

impl<'a> Visitor<'a> for Mysql<'a> {
    const C_BACKTICK_OPEN: &'static str = "`";
    const C_BACKTICK_CLOSE: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let query = query.into();

        let mut this = Mysql {
            query_template: QueryTemplate::new(PlaceholderFormat {
                prefix: "?",
                has_numbering: false,
            }),
            target_table: get_target_table(&query),
        };

        Mysql::visit_query(&mut this, query)?;

        Ok(this.query_template)
    }

    fn write(&mut self, value: impl fmt::Display) -> visitor::Result {
        self.query_template.write_string_chunk(value.to_string());
        Ok(())
    }

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match &value.typed {
            ValueType::Int32(i) => i.map(|i| self.write(i)),
            ValueType::Int64(i) => i.map(|i| self.write(i)),
            ValueType::Float(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f32::INFINITY => self.write("'Infinity'"),
                f if f == f32::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Double(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f64::INFINITY => self.write("'Infinity'"),
                f if f == f64::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{v:?}")),
            }),
            ValueType::Text(t) => t.as_ref().map(|t| self.write(format!("'{t}'"))),
            ValueType::Enum(e, _) => e.as_ref().map(|e| self.write(e)),
            ValueType::Bytes(b) => b.as_ref().map(|b| self.write(format!("x'{}'", hex::encode(b)))),
            ValueType::Boolean(b) => b.map(|b| self.write(b)),
            ValueType::Char(c) => c.map(|c| self.write(format!("'{c}'"))),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                let msg = "Arrays are not supported in MySQL.";
                let kind = ErrorKind::conversion(msg);

                let mut builder = Error::builder(kind);
                builder.set_original_message(msg);

                return Err(builder.build());
            }

            ValueType::Numeric(r) => r.as_ref().map(|r| self.write(r)),

            ValueType::Json(j) => match j {
                Some(j) => {
                    let s = serde_json::to_string(&j)?;
                    Some(self.write(format!("CONVERT('{s}', JSON)")))
                }
                None => None,
            },
            ValueType::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.hyphenated()))),
            ValueType::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            ValueType::Date(date) => date.map(|date| self.write(format!("'{date}'"))),
            ValueType::Time(time) => time.map(|time| self.write(format!("'{time}'"))),
            ValueType::Xml(cow) => cow.as_ref().map(|cow| self.write(format!("'{cow}'"))),

            ValueType::Opaque(opaque) => Some(Err(
                Error::builder(ErrorKind::OpaqueAsRawValue(opaque.to_string())).build()
            )),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT IGNORE ")?,
            _ => self.write("INSERT ")?,
        };

        if let Some(table) = insert.table {
            self.write("INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Parameterized(row),
                ..
            } => {
                let columns = insert.columns.len();

                self.write(" (")?;
                for (i, c) in insert.columns.into_iter().enumerate() {
                    self.visit_column(c.into_bare())?;

                    if i < (columns - 1) {
                        self.write(",")?;
                    }
                }

                self.write(")")?;
                self.write(" VALUES ")?;
                self.query_template.write_parameter_tuple_list("(", ",", ")", ",");
                self.query_template.parameters.push(row);
            }
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" () VALUES ()")?;
                } else {
                    let columns = insert.columns.len();

                    self.write(" (")?;
                    for (i, c) in insert.columns.into_iter().enumerate() {
                        self.visit_column(c.into_bare())?;

                        if i < (columns - 1) {
                            self.write(",")?;
                        }
                    }

                    self.write(")")?;
                    self.write(" VALUES ")?;
                    self.visit_row(row)?;
                }
            }
            Expression {
                kind: ExpressionKind::Values(values),
                ..
            } => {
                let columns = insert.columns.len();

                self.write(" (")?;
                for (i, c) in insert.columns.into_iter().enumerate() {
                    self.visit_column(c.into_bare())?;

                    if i < (columns - 1) {
                        self.write(",")?;
                    }
                }
                self.write(")")?;

                self.write(" VALUES ")?;
                let values_len = values.len();

                for (i, row) in values.into_iter().enumerate() {
                    self.visit_row(row)?;

                    if i < (values_len - 1) {
                        self.write(", ")?;
                    }
                }
            }
            expr => self.surround_with("(", ")", |ref mut s| s.visit_expression(expr))?,
        }

        if let Some(comment) = insert.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }
        Ok(())
    }

    fn visit_upsert(&mut self, _update: crate::ast::Update<'a>) -> visitor::Result {
        unimplemented!("Upsert not supported for the underlying database.")
    }

    /// MySql will error if a `Update` or `Delete` query has a subselect
    /// that references a table that is being updated or deleted
    /// to get around that, we need to wrap the table in a tmp table name
    ///
    /// UPDATE `crabbywilderness` SET `val` = ?
    /// WHERE (`crabbywilderness`.`id`)
    /// IN (SELECT `t1`.`id` FROM `crabbywilderness` AS `t1`
    /// INNER JOIN `breakabletomatoes` AS `j` ON `j`.`id` = `t1`.`id2`)
    fn visit_sub_selection(&mut self, query: SelectQuery<'a>) -> visitor::Result {
        match query {
            SelectQuery::Select(select) => {
                if let Some(table) = &self.target_table
                    && select.tables.contains(table)
                {
                    let tmp_name = "tmp_subselect_table";
                    let tmp_table = Table::from(*select).alias(tmp_name);
                    let sub_select = Select::from_table(tmp_table).value(Table::from(tmp_name).asterisk());

                    return self.visit_select(sub_select);
                }

                self.visit_select(*select)
            }
            SelectQuery::Union(union) => self.visit_union(*union),
        }
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.query_template.parameters.push(value);
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.query_template.write_parameter();
        Ok(())
    }

    fn visit_parameterized_row(
        &mut self,
        value: Value<'a>,
        item_prefix: impl Into<Cow<'static, str>>,
        separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
    ) -> visitor::Result {
        self.query_template
            .write_parameter_tuple(item_prefix, separator, item_suffix);
        self.query_template.parameters.push(value);
        Ok(())
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> visitor::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (
                None,
                Some(Value {
                    typed: ValueType::Int32(Some(offset)),
                    ..
                }),
            ) if offset < 1 => Ok(()),
            (
                None,
                Some(Value {
                    typed: ValueType::Int64(Some(offset)),
                    ..
                }),
            ) if offset < 1 => Ok(()),
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(Value::from(9_223_372_036_854_775_807i64))?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (Some(limit), None) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> visitor::Result {
        self.write(" GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        {
            if right.is_json_expr() || left.is_json_expr() {
                self.surround_with("(", ")", |ref mut s| {
                    s.write("JSON_CONTAINS")?;
                    s.surround_with("(", ")", |s| {
                        s.visit_expression(left.clone())?;
                        s.write(", ")?;
                        s.visit_expression(right.clone())
                    })?;

                    s.write(" AND ")?;

                    s.write("JSON_CONTAINS")?;
                    s.surround_with("(", ")", |s| {
                        s.visit_expression(right)?;
                        s.write(", ")?;
                        s.visit_expression(left)
                    })
                })
            } else {
                self.visit_regular_equality_comparison(left, right)
            }
        }
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        {
            if right.is_json_expr() || left.is_json_expr() {
                self.surround_with("(", ")", |ref mut s| {
                    s.write("NOT JSON_CONTAINS")?;
                    s.surround_with("(", ")", |s| {
                        s.visit_expression(left.clone())?;
                        s.write(", ")?;
                        s.visit_expression(right.clone())
                    })?;

                    s.write(" OR ")?;

                    s.write("NOT JSON_CONTAINS")?;
                    s.surround_with("(", ")", |s| {
                        s.visit_expression(right)?;
                        s.write(", ")?;
                        s.visit_expression(left)
                    })
                })
            } else {
                self.visit_regular_difference_comparison(left, right)
            }
        }
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitor::Result {
        if json_extract.extract_as_string {
            self.write("JSON_UNQUOTE(")?;
        }

        self.write("JSON_EXTRACT(")?;
        self.visit_expression(*json_extract.column)?;
        self.write(", ")?;

        match json_extract.path.clone() {
            JsonPath::Array(_) => panic!("JSON path array notation is not supported for MySQL"),
            JsonPath::String(path) => self.visit_parameterized(Value::text(path))?,
        }

        self.write(")")?;

        if json_extract.extract_as_string {
            self.write(")")?;
        }

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_array_contains(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> visitor::Result {
        self.write("JSON_CONTAINS(")?;
        self.visit_expression(left)?;
        self.write(", ")?;
        self.visit_expression(right)?;
        self.write(")")?;

        if not {
            self.write(" = FALSE")?;
        }

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_type_equals(&mut self, left: Expression<'a>, json_type: JsonType<'a>, not: bool) -> visitor::Result {
        self.write("(")?;
        self.write("JSON_TYPE")?;
        self.surround_with("(", ")", |s| s.visit_expression(left.clone()))?;

        if not {
            self.write(" != ")?;
        } else {
            self.write(" = ")?;
        }

        match json_type {
            JsonType::Array => {
                self.visit_expression(Expression::from(Value::text("ARRAY")))?;
            }
            JsonType::Boolean => {
                self.visit_expression(Expression::from(Value::text("BOOLEAN")))?;
            }
            JsonType::Number => {
                self.visit_expression(Expression::from(Value::text("INTEGER")))?;
                self.write(" OR JSON_TYPE(")?;
                self.visit_expression(left)?;
                self.write(")")?;
                self.write(" = ")?;
                self.visit_expression(Expression::from(Value::text("DOUBLE")))?;
            }
            JsonType::Object => {
                self.visit_expression(Expression::from(Value::text("OBJECT")))?;
            }
            JsonType::String => {
                self.visit_expression(Expression::from(Value::text("STRING")))?;
            }
            JsonType::Null => {
                self.visit_expression(Expression::from(Value::text("NULL")))?;
            }
            JsonType::ColumnRef(column) => {
                self.write("JSON_TYPE")?;
                self.surround_with("(", ")", |s| s.visit_column(*column))?;
            }
        }

        self.write(")")
    }

    fn visit_greater_than(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_numeric_comparison(left, right, ">")?;

        Ok(())
    }

    fn visit_greater_than_or_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_numeric_comparison(left, right, ">=")?;

        Ok(())
    }

    fn visit_less_than(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_numeric_comparison(left, right, "<")?;

        Ok(())
    }

    fn visit_less_than_or_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        self.visit_numeric_comparison(left, right, "<=")?;

        Ok(())
    }

    fn visit_text_search(&mut self, text_search: crate::prelude::TextSearch<'a>) -> visitor::Result {
        let len = text_search.exprs.len();
        self.surround_with("MATCH (", ")", |s| {
            for (i, expr) in text_search.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }

            Ok(())
        })
    }

    fn visit_matches(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> visitor::Result {
        if not {
            self.write("(NOT ")?;
        }

        self.visit_expression(left)?;
        self.surround_with("AGAINST (", " IN BOOLEAN MODE)", |s| s.visit_expression(right))?;

        if not {
            self.write(")")?;
        }

        Ok(())
    }

    fn visit_text_search_relevance(&mut self, text_search_relevance: TextSearchRelevance<'a>) -> visitor::Result {
        let exprs = text_search_relevance.exprs;
        let query = text_search_relevance.query;

        let text_search = TextSearch { exprs };

        self.visit_expression(text_search.into())?;
        self.surround_with("AGAINST (", " IN BOOLEAN MODE)", |s| s.visit_expression(query))?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> visitor::Result {
        self.write("JSON_EXTRACT(")?;
        self.visit_expression(*extract.expr.clone())?;
        self.write(", ")?;
        self.write("CONCAT('$[', ")?;
        self.write("JSON_LENGTH(")?;
        self.visit_expression(*extract.expr)?;
        self.write(") - 1, ']'))")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> visitor::Result {
        self.write("JSON_EXTRACT(")?;
        self.visit_expression(*extract.expr)?;
        self.write(", ")?;
        self.visit_parameterized(Value::text("$[0]"))?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_unquote(&mut self, json_unquote: JsonUnquote<'a>) -> visitor::Result {
        self.write("JSON_UNQUOTE(")?;
        self.visit_expression(*json_unquote.expr)?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(feature = "mysql")]
    fn visit_json_array_agg(&mut self, array_agg: JsonArrayAgg<'a>) -> visitor::Result {
        self.write("JSON_ARRAYAGG")?;
        self.surround_with("(", ")", |s| s.visit_expression(*array_agg.expr))?;

        Ok(())
    }

    #[cfg(feature = "mysql")]
    fn visit_json_build_object(&mut self, build_obj: JsonBuildObject<'a>) -> visitor::Result {
        let len = build_obj.exprs.len();

        self.write("JSON_OBJECT")?;
        self.surround_with("(", ")", |s| {
            for (i, (name, expr)) in build_obj.exprs.into_iter().enumerate() {
                s.visit_raw_value(Value::text(name))?;
                s.write(", ")?;
                s.visit_json_build_obj_expr(expr)?;

                if i < (len - 1) {
                    s.write(", ")?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> visitor::Result {
        let len = ordering.0.len();

        // ORDER BY <value> IS NOT NULL, <value> <direction> = NULLS FIRST
        // ORDER BY <value> IS NULL, <value> <direction> = NULLS LAST
        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            match ordering {
                Some(Order::Asc) => {
                    self.visit_order_by("ASC", value)?;
                }
                Some(Order::Desc) => {
                    self.visit_order_by("DESC", value)?;
                }
                Some(Order::AscNullsFirst) => {
                    self.visit_order_by("IS NOT NULL", value.clone())?;
                    self.write(", ")?;
                    self.visit_order_by("ASC", value)?;
                }
                Some(Order::AscNullsLast) => {
                    self.visit_order_by("IS NULL", value.clone())?;
                    self.write(", ")?;
                    self.visit_order_by("ASC", value)?;
                }
                Some(Order::DescNullsFirst) => {
                    self.visit_order_by("IS NOT NULL", value.clone())?;
                    self.write(", ")?;
                    self.visit_order_by("DESC", value)?;
                }
                Some(Order::DescNullsLast) => {
                    self.visit_order_by("IS NULL", value.clone())?;
                    self.write(", ")?;
                    self.visit_order_by("DESC", value)?;
                }
                None => {
                    self.visit_expression(value)?;
                }
            };

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }
}

fn get_target_table<'a>(query: &Query<'a>) -> Option<Table<'a>> {
    match query {
        Query::Delete(delete) => Some(delete.table.clone()),
        Query::Update(update) => Some(update.table.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::visitor::*;

    fn expected_values<'a, T>(sql: &'static str, params: Vec<T>) -> (String, Vec<Value<'a>>)
    where
        T: Into<Value<'a>>,
    {
        (String::from(sql), params.into_iter().map(|p| p.into()).collect())
    }

    fn default_params(mut additional: Vec<Value<'_>>) -> Vec<Value<'_>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_single_row_insert_default_values() {
        let query = Insert::single_into("users");
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!("INSERT INTO `users` () VALUES ()", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?)", vec![10]);
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values("INSERT INTO `users` (`foo`) VALUES (?), (?)", vec![10, 11]);
        let query = Insert::multi_into("users", vec!["foo"])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ? OFFSET ?", vec![10_i64, 2_i64]);
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values(
            "SELECT `users`.* FROM `users` LIMIT ? OFFSET ?",
            vec![9_223_372_036_854_775_807i64, 10],
        );

        let query = Select::from_table("users").offset(10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT `users`.* FROM `users` LIMIT ?", vec![10_i64]);
        let query = Select::from_table("users").limit(10);
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_in_values_2_tuple() {
        let expected_sql = "SELECT `test`.* FROM `test` WHERE (`id1`,`id2`) IN ((?,?),(?,?))";
        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![Value::int32(1), Value::int32(2), Value::int32(3), Value::int32(4),],
            params
        );
    }

    #[test]
    fn json_build_object_casts_bigint_to_string() {
        let build_json = json_build_object(vec![(
            "id".into(),
            Column::from("id")
                .native_column_type(Some("BIGINT"))
                .type_family(TypeFamily::Int)
                .into(),
        )]);
        let query = Select::default().value(build_json);
        let (sql, _) = Mysql::build(query).unwrap();

        assert_eq!("SELECT JSON_OBJECT('id', CONVERT(`id`, CHAR))", sql);
    }

    #[test]
    fn json_build_object_casts_unsigned_bigint_to_string() {
        let build_json = json_build_object(vec![(
            "id".into(),
            Column::from("id")
                .native_column_type(Some("UNSIGNEDBIGINT"))
                .type_family(TypeFamily::Int)
                .into(),
        )]);
        let query = Select::default().value(build_json);
        let (sql, _) = Mysql::build(query).unwrap();

        assert_eq!("SELECT JSON_OBJECT('id', CONVERT(`id`, CHAR))", sql);
    }

    #[test]
    fn equality_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT `users`.* FROM `users` WHERE (JSON_CONTAINS(`jsonField`, ?) AND JSON_CONTAINS(?, `jsonField`))"#,
            vec![serde_json::json!({"a": "b"}), serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table("users").so_that(Column::from("jsonField").equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT `users`.* FROM `users` WHERE (NOT JSON_CONTAINS(`jsonField`, ?) OR NOT JSON_CONTAINS(?, `jsonField`))"#,
            vec![serde_json::json!({"a": "b"}), serde_json::json!({"a": "b"})],
        );

        let query =
            Select::from_table("users").so_that(Column::from("jsonField").not_equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Mysql::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Mysql::build(Select::default().value(ValueType::Text(None).raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Mysql::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Mysql::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Mysql::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Mysql::build(Select::default().value(ValueType::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT x'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Mysql::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Mysql::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Mysql::build(Select::default().value(ValueType::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT `bar` FROM `test`";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Mysql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT ? FROM `test2`), `bar` FROM `test`";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Mysql::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_from() {
        let expected_sql = "SELECT `foo`.*, `bar`.`a` FROM `foo`, (SELECT `a` FROM `baz`) AS `bar`";
        let query = Select::default()
            .and_from("foo")
            .and_from(Table::from(Select::from_table("baz").column("a")).alias("bar"))
            .value(Table::from("foo").asterisk())
            .column(("bar", "a"));

        let (sql, _) = Mysql::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_insert() {
        let expected_sql = "INSERT INTO `users` () VALUES () /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Insert::single_into("users");
        let insert =
            Insert::from(query).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Mysql::build(insert).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]

    fn test_raw_json() {
        let (sql, params) = Mysql::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT CONVERT('{\"foo\":\"bar\"}', JSON)", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Mysql::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", uuid.hyphenated()), sql);

        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Mysql::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into("foo")
            .value("foo", "bar")
            .value("baz", default_value());

        let (sql, _) = Mysql::build(insert).unwrap();

        assert_eq!("INSERT INTO `foo` (`foo`,`baz`) VALUES (?,DEFAULT)", sql);
    }

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table = Table::from("User").left_join(
            "Post"
                .alias("p")
                .on(("p", "userId").equals(Column::from(("User", "id")))),
        );
        let q = Select::from_table(joined_table).and_from("Toto");
        let (sql, _) = Mysql::build(q).unwrap();

        assert_eq!(
            "SELECT `User`.*, `Toto`.* FROM `User` LEFT JOIN `Post` AS `p` ON `p`.`userId` = `User`.`id`, `Toto`",
            sql
        );
    }

    #[test]

    fn test_json_negation() {
        let conditions = ConditionTree::not("json".equals(ValueType::Json(Some(serde_json::Value::Null))));
        let (sql, _) = Mysql::build(Select::from_table("test").so_that(conditions)).unwrap();

        assert_eq!(
            "SELECT `test`.* FROM `test` WHERE (NOT (JSON_CONTAINS(`json`, ?) AND JSON_CONTAINS(?, `json`)))",
            sql
        );
    }

    #[test]

    fn test_json_not_negation() {
        let conditions = ConditionTree::not("json".not_equals(ValueType::Json(Some(serde_json::Value::Null))));
        let (sql, _) = Mysql::build(Select::from_table("test").so_that(conditions)).unwrap();

        assert_eq!(
            "SELECT `test`.* FROM `test` WHERE (NOT (NOT JSON_CONTAINS(`json`, ?) OR NOT JSON_CONTAINS(?, `json`)))",
            sql
        );
    }

    #[test]
    fn test_subselect_temp_table_wrapper_for_update() {
        let table_1 = "table_1";
        let table_2 = "table2";

        let join = table_2.alias("j").on(("j", "id").equals(Column::from(("t1", "id2"))));
        let a = table_1.alias("t1");
        let selection = Select::from_table(a).column(("t1", "id")).inner_join(join);

        let id1 = Column::from((table_1, "id"));
        let conditions = Row::from(vec![id1]).in_selection(selection);
        let update = Update::table(table_1).set("val", 2).so_that(conditions);

        let (sql, _) = Mysql::build(update).unwrap();

        assert_eq!(
            "UPDATE `table_1` SET `val` = ? WHERE (`table_1`.`id`) IN (SELECT `tmp_subselect_table`.* FROM (SELECT `t1`.`id` FROM `table_1` AS `t1` INNER JOIN `table2` AS `j` ON `j`.`id` = `t1`.`id2`) AS `tmp_subselect_table`)",
            sql
        );
    }

    #[test]
    fn test_subselect_temp_table_wrapper_for_delete() {
        let table_1 = "table_1";
        let table_2 = "table2";

        let join = table_2.alias("j").on(("j", "id").equals(Column::from(("t1", "id2"))));
        let a = table_1.alias("t1");
        let selection = Select::from_table(a).column(("t1", "id")).inner_join(join);

        let id1 = Column::from((table_1, "id"));
        let conditions = Row::from(vec![id1]).in_selection(selection);
        let update = Delete::from_table(table_1).so_that(conditions);

        let (sql, _) = Mysql::build(update).unwrap();

        assert_eq!(
            "DELETE FROM `table_1` WHERE (`table_1`.`id`) IN (SELECT `tmp_subselect_table`.* FROM (SELECT `t1`.`id` FROM `table_1` AS `t1` INNER JOIN `table2` AS `j` ON `j`.`id` = `t1`.`id2`) AS `tmp_subselect_table`)",
            sql
        );
    }
}
