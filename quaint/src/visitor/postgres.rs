use crate::{
    ast::*,
    visitor::{self, Visitor},
};
use std::{
    fmt::{self, Write},
    ops::Deref,
};

/// A visitor to generate queries for the PostgreSQL database.
///
/// The returned parameter values implement the `ToSql` trait from postgres and
/// can be used directly with the database.
pub struct Postgres<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Visitor<'a> for Postgres<'a> {
    const C_BACKTICK_OPEN: &'static str = "\"";
    const C_BACKTICK_CLOSE: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> crate::Result<(String, Vec<Value<'a>>)>
    where
        Q: Into<Query<'a>>,
    {
        let mut postgres = Postgres {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Postgres::visit_query(&mut postgres, query.into())?;

        Ok((postgres.query, postgres.parameters))
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> visitor::Result {
        write!(&mut self.query, "{s}")?;
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.write("$")?;
        self.write(self.parameters.len())
    }

    fn visit_parameterized_enum(&mut self, variant: EnumVariant<'a>, name: Option<EnumName<'a>>) -> visitor::Result {
        self.add_parameter(variant.into_text());

        // Since enums are user-defined custom types, tokio-postgres fires an additional query
        // when parameterizing values of type enum to know which custom type the value refers to.
        // Casting the enum value to `TEXT` avoid this roundtrip since `TEXT` is a builtin type.
        if let Some(enum_name) = name {
            self.surround_with("CAST(", ")", |ref mut s| {
                s.parameter_substitution()?;
                s.write("::text")?;
                s.write(" AS ")?;
                if let Some(schema_name) = enum_name.schema_name {
                    s.surround_with_backticks(schema_name.deref())?;
                    s.write(".")?
                }
                s.surround_with_backticks(enum_name.name.deref())
            })?;
        } else {
            self.parameter_substitution()?;
        }

        Ok(())
    }

    fn visit_parameterized_enum_array(
        &mut self,
        variants: Vec<EnumVariant<'a>>,
        name: Option<EnumName<'a>>,
    ) -> visitor::Result {
        // Since enums are user-defined custom types, tokio-postgres fires an additional query
        // when parameterizing values of type enum to know which custom type the value refers to.
        // Casting the enum value to `TEXT` avoid this roundtrip since `TEXT` is a builtin type.
        if let Some(enum_name) = name.clone() {
            self.add_parameter(Value::array(variants.into_iter().map(|v| v.into_text())));

            self.surround_with("CAST(", ")", |s| {
                s.parameter_substitution()?;
                s.write("::text[]")?;
                s.write(" AS ")?;

                if let Some(schema_name) = enum_name.schema_name {
                    s.surround_with_backticks(schema_name.deref())?;
                    s.write(".")?
                }

                s.surround_with_backticks(enum_name.name.deref())?;
                s.write("[]")?;

                Ok(())
            })?;
        } else {
            self.visit_parameterized(Value::array(
                variants.into_iter().map(|variant| variant.into_enum(name.clone())),
            ))?;
        }

        Ok(())
    }

    /// A database column identifier
    fn visit_column(&mut self, column: Column<'a>) -> visitor::Result {
        match column.table {
            Some(table) => {
                self.visit_table(table, false)?;
                self.write(".")?;
                self.delimited_identifiers(&[&*column.name])?;
            }
            _ => self.delimited_identifiers(&[&*column.name])?,
        };

        if column.is_enum && column.is_selected {
            if column.is_list {
                self.write("::text[]")?;
            } else {
                self.write("::text")?;
            }
        }

        if let Some(alias) = column.alias {
            self.write(" AS ")?;
            self.delimited_identifiers(&[&*alias])?;
        }

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
            (None, Some(offset)) => {
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

    fn visit_raw_value(&mut self, value: Value<'a>) -> visitor::Result {
        let res = match &value.typed {
            ValueType::Int32(i) => i.map(|i| self.write(i)),
            ValueType::Int64(i) => i.map(|i| self.write(i)),
            ValueType::Text(t) => t.as_ref().map(|t| self.write(format!("'{t}'"))),
            ValueType::Enum(e, _) => e.as_ref().map(|e| self.write(e)),
            ValueType::Bytes(b) => b.as_ref().map(|b| self.write(format!("E'{}'", hex::encode(b)))),
            ValueType::Boolean(b) => b.map(|b| self.write(b)),
            ValueType::Xml(cow) => cow.as_ref().map(|cow| self.write(format!("'{cow}'"))),
            ValueType::Char(c) => c.map(|c| self.write(format!("'{c}'"))),
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
            ValueType::Array(ary) => ary.as_ref().map(|ary| {
                self.surround_with("'{", "}'", |ref mut s| {
                    let len = ary.len();

                    for (i, item) in ary.iter().enumerate() {
                        s.write(item)?;

                        if i < len - 1 {
                            s.write(",")?;
                        }
                    }

                    Ok(())
                })
            }),
            ValueType::EnumArray(variants, name) => variants.as_ref().map(|variants| {
                self.surround_with("ARRAY[", "]", |ref mut s| {
                    let len = variants.len();

                    for (i, item) in variants.iter().enumerate() {
                        s.surround_with("'", "'", |t| t.write(item))?;

                        if i < len - 1 {
                            s.write(",")?;
                        }
                    }

                    Ok(())
                })?;

                if let Some(enum_name) = name {
                    self.write("::")?;
                    if let Some(schema_name) = &enum_name.schema_name {
                        self.surround_with_backticks(schema_name.deref())?;
                        self.write(".")?
                    }
                    self.surround_with_backticks(enum_name.name.deref())?;
                }

                Ok(())
            }),
            ValueType::Json(j) => j
                .as_ref()
                .map(|j| self.write(format!("'{}'", serde_json::to_string(&j).unwrap()))),

            ValueType::Numeric(r) => r.as_ref().map(|r| self.write(r)),
            ValueType::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.hyphenated()))),
            ValueType::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            ValueType::Date(date) => date.map(|date| self.write(format!("'{date}'"))),
            ValueType::Time(time) => time.map(|time| self.write(format!("'{time}'"))),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        self.write("INSERT ")?;

        if let Some(table) = insert.table.clone() {
            self.write("INTO ")?;
            self.visit_table(table, true)?;
        }

        match insert.values {
            Expression {
                kind: ExpressionKind::Row(row),
                ..
            } => {
                if row.values.is_empty() {
                    self.write(" DEFAULT VALUES")?;
                } else {
                    let columns = insert.columns.len();

                    self.write(" (")?;
                    for (i, c) in insert.columns.into_iter().enumerate() {
                        self.visit_column(c.name.into_owned().into())?;

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
                    self.visit_column(c.name.into_owned().into())?;

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

        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write(" ON CONFLICT DO NOTHING")?,
            Some(OnConflict::Update(update, constraints)) => {
                self.write(" ON CONFLICT")?;
                self.columns_to_bracket_list(constraints)?;
                self.write(" DO ")?;

                self.visit_upsert(update)?;
            }
            None => (),
        }

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values = returning.into_iter().map(|r| r.into()).collect();
                self.write(" RETURNING ")?;
                self.visit_columns(values)?;
            }
        };

        if let Some(comment) = insert.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> visitor::Result {
        self.write("ARRAY_TO_STRING")?;
        self.write("(")?;
        self.write("ARRAY_AGG")?;
        self.write("(")?;
        self.visit_expression(value)?;
        self.write(")")?;
        self.write("','")?;
        self.write(")")
    }

    fn visit_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        // LHS must be cast to json/xml-text if the right is a json/xml-text value and vice versa.
        let right_cast = match left {
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            _ if right.is_json_value() => "::jsonb",
            _ if right.is_xml_value() => "::text",
            _ => "",
        };

        self.visit_expression(left)?;
        self.write(left_cast)?;
        self.write(" = ")?;
        self.visit_expression(right)?;
        self.write(right_cast)?;

        Ok(())
    }

    fn visit_not_equals(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        // LHS must be cast to json/xml-text if the right is a json/xml-text value and vice versa.
        let right_cast = match left {
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            _ if right.is_json_value() => "::jsonb",
            _ if right.is_xml_value() => "::text",
            _ => "",
        };

        self.visit_expression(left)?;
        self.write(left_cast)?;
        self.write(" <> ")?;
        self.visit_expression(right)?;
        self.write(right_cast)?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitor::Result {
        match json_extract.path {
            #[cfg(feature = "mysql")]
            JsonPath::String(_) => panic!("JSON path string notation is not supported for Postgres"),
            JsonPath::Array(json_path) => {
                self.write("(")?;
                self.visit_expression(*json_extract.column)?;

                if json_extract.extract_as_string {
                    self.write("#>>")?;
                } else {
                    self.write("#>")?;
                }

                // We use the `ARRAY[]::text[]` notation to better handle escaped character
                // The text protocol used when sending prepared statement doesn't seem to work well with escaped characters
                // when using the '{a, b, c}' string array notation.
                self.surround_with("ARRAY[", "]::text[]", |s| {
                    let len = json_path.len();
                    for (index, path) in json_path.into_iter().enumerate() {
                        s.visit_parameterized(Value::text(path))?;
                        if index < len - 1 {
                            s.write(", ")?;
                        }
                    }
                    Ok(())
                })?;

                self.write(")")?;

                if !json_extract.extract_as_string {
                    self.write("::jsonb")?;
                }
            }
        }

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_unquote(&mut self, json_unquote: JsonUnquote<'a>) -> visitor::Result {
        self.write("(")?;
        self.visit_expression(*json_unquote.expr)?;
        self.write("#>>ARRAY[]::text[]")?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_array_contains(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> visitor::Result {
        if not {
            self.write("( NOT ")?;
        }

        self.visit_expression(left)?;
        self.write(" @> ")?;
        self.visit_expression(right)?;

        if not {
            self.write(" )")?;
        }

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> visitor::Result {
        self.write("(")?;
        self.visit_expression(*extract.expr)?;
        self.write("->-1")?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> visitor::Result {
        self.write("(")?;
        self.visit_expression(*extract.expr)?;
        self.write("->0")?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql"))]
    fn visit_json_type_equals(&mut self, left: Expression<'a>, json_type: JsonType<'a>, not: bool) -> visitor::Result {
        self.write("JSONB_TYPEOF")?;
        self.write("(")?;
        self.visit_expression(left)?;
        self.write(")")?;

        if not {
            self.write(" != ")?;
        } else {
            self.write(" = ")?;
        }

        match json_type {
            JsonType::Array => self.visit_expression(Value::text("array").into()),
            JsonType::Boolean => self.visit_expression(Value::text("boolean").into()),
            JsonType::Number => self.visit_expression(Value::text("number").into()),
            JsonType::Object => self.visit_expression(Value::text("object").into()),
            JsonType::String => self.visit_expression(Value::text("string").into()),
            JsonType::Null => self.visit_expression(Value::text("null").into()),
            JsonType::ColumnRef(column) => {
                self.write("JSONB_TYPEOF")?;
                self.write("(")?;
                self.visit_column(*column)?;
                self.write("::jsonb)")
            }
        }
    }

    fn visit_text_search(&mut self, text_search: crate::prelude::TextSearch<'a>) -> visitor::Result {
        let len = text_search.exprs.len();
        self.surround_with("to_tsvector(concat_ws(' ', ", "))", |s| {
            for (i, expr) in text_search.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }

            Ok(())
        })
    }

    fn visit_matches(&mut self, left: Expression<'a>, right: std::borrow::Cow<'a, str>, not: bool) -> visitor::Result {
        if not {
            self.write("(NOT ")?;
        }

        self.visit_expression(left)?;
        self.write(" @@ ")?;
        self.surround_with("to_tsquery(", ")", |s| s.visit_parameterized(Value::text(right)))?;

        if not {
            self.write(")")?;
        }

        Ok(())
    }

    fn visit_text_search_relevance(&mut self, text_search_relevance: TextSearchRelevance<'a>) -> visitor::Result {
        let len = text_search_relevance.exprs.len();
        let exprs = text_search_relevance.exprs;
        let query = text_search_relevance.query;

        self.write("ts_rank(")?;
        self.surround_with("to_tsvector(concat_ws(' ', ", "))", |s| {
            for (i, expr) in exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }

            Ok(())
        })?;
        self.write(", ")?;
        self.surround_with("to_tsquery(", ")", |s| s.visit_parameterized(Value::text(query)))?;
        self.write(")")?;

        Ok(())
    }

    fn visit_like(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        let need_cast = matches!(&left.kind, ExpressionKind::Column(_));
        self.visit_expression(left)?;

        // NOTE: Pg is strongly typed, LIKE comparisons are only between strings.
        // to avoid problems with types without implicit casting we explicitly cast to text
        if need_cast {
            self.write("::text")?;
        }

        self.write(" LIKE ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_not_like(&mut self, left: Expression<'a>, right: Expression<'a>) -> visitor::Result {
        let need_cast = matches!(&left.kind, ExpressionKind::Column(_));
        self.visit_expression(left)?;

        // NOTE: Pg is strongly typed, LIKE comparisons are only between strings.
        // to avoid problems with types without implicit casting we explicitly cast to text
        if need_cast {
            self.write("::text")?;
        }

        self.write(" NOT LIKE ")?;
        self.visit_expression(right)?;

        Ok(())
    }

    fn visit_ordering(&mut self, ordering: Ordering<'a>) -> visitor::Result {
        let len = ordering.0.len();

        for (i, (value, ordering)) in ordering.0.into_iter().enumerate() {
            let direction = ordering.map(|dir| match dir {
                Order::Asc => " ASC",
                Order::Desc => " DESC",
                Order::AscNullsFirst => "ASC NULLS FIRST",
                Order::AscNullsLast => "ASC NULLS LAST",
                Order::DescNullsFirst => "DESC NULLS FIRST",
                Order::DescNullsLast => "DESC NULLS LAST",
            });

            self.visit_expression(value)?;
            self.write(direction.unwrap_or(""))?;

            if i < (len - 1) {
                self.write(", ")?;
            }
        }

        Ok(())
    }

    fn visit_concat(&mut self, concat: Concat<'a>) -> visitor::Result {
        let len = concat.exprs.len();

        self.surround_with("(", ")", |s| {
            for (i, expr) in concat.exprs.into_iter().enumerate() {
                s.visit_expression(expr)?;

                if i < (len - 1) {
                    s.write(" || ")?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!("INSERT INTO \"users\" DEFAULT VALUES", sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_single_row_insert() {
        let expected = expected_values("INSERT INTO \"users\" (\"foo\") VALUES ($1)", vec![10]);
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    #[cfg(feature = "postgresql")]
    fn test_returning_insert() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"foo\") VALUES ($1) RETURNING \"foo\"",
            vec![10],
        );
        let query = Insert::single_into("users").value("foo", 10);
        let (sql, params) = Postgres::build(Insert::from(query).returning(vec!["foo"])).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    #[cfg(feature = "postgresql")]
    fn test_insert_on_conflict_update() {
        let expected = expected_values(
            "INSERT INTO \"users\" (\"foo\") VALUES ($1) ON CONFLICT (\"foo\") DO UPDATE SET \"foo\" = $2 WHERE \"users\".\"foo\" = $3 RETURNING \"foo\"",
            vec![10, 3, 1],
        );

        let update = Update::table("users").set("foo", 3).so_that(("users", "foo").equals(1));

        let query: Insert = Insert::single_into("users").value("foo", 10).into();

        let query = query.on_conflict(OnConflict::Update(update, Vec::from(["foo".into()])));

        let (sql, params) = Postgres::build(query.returning(vec!["foo"])).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_multi_row_insert() {
        let expected = expected_values("INSERT INTO \"users\" (\"foo\") VALUES ($1), ($2)", vec![10, 11]);
        let query = Insert::multi_into("users", vec!["foo"])
            .values(vec![10])
            .values(vec![11]);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_both_are_set() {
        let expected = expected_values(
            "SELECT \"users\".* FROM \"users\" LIMIT $1 OFFSET $2",
            vec![10_i64, 2_i64],
        );
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" OFFSET $1", vec![10_i64]);
        let query = Select::from_table("users").offset(10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1", vec![10_i64]);
        let query = Select::from_table("users").limit(10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_distinct() {
        let expected_sql = "SELECT DISTINCT \"bar\" FROM \"test\"";
        let query = Select::from_table("test").column(Column::new("bar")).distinct();
        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_distinct_with_subquery() {
        let expected_sql = "SELECT DISTINCT (SELECT $1 FROM \"test2\"), \"bar\" FROM \"test\"";
        let query = Select::from_table("test")
            .value(Select::from_table("test2").value(val!(1)))
            .column(Column::new("bar"))
            .distinct();

        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_from() {
        let expected_sql = "SELECT \"foo\".*, \"bar\".\"a\" FROM \"foo\", (SELECT \"a\" FROM \"baz\") AS \"bar\"";
        let query = Select::default()
            .and_from("foo")
            .and_from(Table::from(Select::from_table("baz").column("a")).alias("bar"))
            .value(Table::from("foo").asterisk())
            .column(("bar", "a"));

        let (sql, _) = Postgres::build(query).unwrap();
        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_select() {
        let expected_sql = "SELECT \"users\".* FROM \"users\" /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Select::from_table("users")
            .comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_insert() {
        let expected_sql = "INSERT INTO \"users\" DEFAULT VALUES /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Insert::single_into("users");
        let insert =
            Insert::from(query).comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Postgres::build(insert).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_update() {
        let expected_sql = "UPDATE \"users\" SET \"foo\" = $1 /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Update::table("users")
            .set("foo", 10)
            .comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_comment_delete() {
        let expected_sql =
            "DELETE FROM \"users\" /* trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2' */";
        let query = Delete::from_table("users")
            .comment("trace_id='5bd66ef5095369c7b0d1f8f4bd33716a', parent_id='c532cb4098ac3dd2'");

        let (sql, _) = Postgres::build(query).unwrap();

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn equality_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "jsonField"::jsonb = $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query = Select::from_table("users").so_that(Column::from("jsonField").equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_lhs_json_value() {
        // A bit artificial, but checks if the ::jsonb casting is done correctly on the right side as well.
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 = "jsonField"::jsonb"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let value_expr: Expression = Value::json(serde_json::json!({"a":"b"})).into();
        let query = Select::from_table("users").so_that(value_expr.equals(Column::from("jsonField")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "jsonField"::jsonb <> $1"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let query =
            Select::from_table("users").so_that(Column::from("jsonField").not_equals(serde_json::json!({"a":"b"})));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_lhs_json_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 <> "jsonField"::jsonb"#,
            vec![serde_json::json!({"a": "b"})],
        );

        let value_expr: Expression = Value::json(serde_json::json!({"a":"b"})).into();
        let query = Select::from_table("users").so_that(value_expr.not_equals(Column::from("jsonField")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "xmlField"::text = $1"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let query =
            Select::from_table("users").so_that(Column::from("xmlField").equals(Value::xml("<salad>wurst</salad>")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn equality_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 = "xmlField"::text"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let value_expr: Expression = Value::xml("<salad>wurst</salad>").into();
        let query = Select::from_table("users").so_that(value_expr.equals(Column::from("xmlField")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE "xmlField"::text <> $1"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let query = Select::from_table("users")
            .so_that(Column::from("xmlField").not_equals(Value::xml("<salad>wurst</salad>")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn difference_with_a_lhs_xml_value() {
        let expected = expected_values(
            r#"SELECT "users".* FROM "users" WHERE $1 <> "xmlField"::text"#,
            vec![Value::xml("<salad>wurst</salad>")],
        );

        let value_expr: Expression = Value::xml("<salad>wurst</salad>").into();
        let query = Select::from_table("users").so_that(value_expr.not_equals(Column::from("xmlField")));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_raw_null() {
        let (sql, params) = Postgres::build(Select::default().value(Value::null_text().raw())).unwrap();
        assert_eq!("SELECT null", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_int() {
        let (sql, params) = Postgres::build(Select::default().value(1.raw())).unwrap();
        assert_eq!("SELECT 1", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_real() {
        let (sql, params) = Postgres::build(Select::default().value(1.3f64.raw())).unwrap();
        assert_eq!("SELECT 1.3", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_text() {
        let (sql, params) = Postgres::build(Select::default().value("foo".raw())).unwrap();
        assert_eq!("SELECT 'foo'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_bytes() {
        let (sql, params) = Postgres::build(Select::default().value(Value::bytes(vec![1, 2, 3]).raw())).unwrap();
        assert_eq!("SELECT E'010203'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_boolean() {
        let (sql, params) = Postgres::build(Select::default().value(true.raw())).unwrap();
        assert_eq!("SELECT true", sql);
        assert!(params.is_empty());

        let (sql, params) = Postgres::build(Select::default().value(false.raw())).unwrap();
        assert_eq!("SELECT false", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_char() {
        let (sql, params) = Postgres::build(Select::default().value(Value::character('a').raw())).unwrap();
        assert_eq!("SELECT 'a'", sql);
        assert!(params.is_empty());
    }

    #[test]

    fn test_raw_json() {
        let (sql, params) =
            Postgres::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Postgres::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", uuid.hyphenated()), sql);

        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_datetime() {
        let dt = chrono::Utc::now();
        let (sql, params) = Postgres::build(Select::default().value(dt.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", dt.to_rfc3339(),), sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_raw_comparator() {
        let (sql, _) = Postgres::build(Select::from_table("foo").so_that("bar".compare_raw("ILIKE", "baz%"))).unwrap();

        assert_eq!(r#"SELECT "foo".* FROM "foo" WHERE "bar" ILIKE $1"#, sql);
    }

    #[test]
    fn test_raw_enum_array() {
        let enum_array = Value::enum_array_with_name(
            vec![EnumVariant::new("A"), EnumVariant::new("B")],
            EnumName::new("Alphabet", Some("foo")),
        );
        let (sql, params) = Postgres::build(Select::default().value(enum_array.raw())).unwrap();

        assert_eq!("SELECT ARRAY['A','B']::\"foo\".\"Alphabet\"", sql);
        assert!(params.is_empty());
    }

    #[test]
    fn test_like_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text LIKE $1"#,
            vec!["%foo%"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").like("%foo%"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_not_like_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text NOT LIKE $1"#,
            vec!["%foo%"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").not_like("%foo%"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_begins_with_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text LIKE $1"#,
            vec!["%foo"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").like("%foo"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_not_begins_with_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text NOT LIKE $1"#,
            vec!["%foo"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").not_like("%foo"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_ends_with_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text LIKE $1"#,
            vec!["foo%"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").like("foo%"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_not_ends_with_cast_to_string() {
        let expected = expected_values(
            r#"SELECT "test".* FROM "test" WHERE "jsonField"::text NOT LIKE $1"#,
            vec!["foo%"],
        );

        let query = Select::from_table("test").so_that(Column::from("jsonField").not_like("foo%"));
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_default_insert() {
        let insert = Insert::single_into("foo")
            .value("foo", "bar")
            .value("baz", default_value());

        let (sql, _) = Postgres::build(insert).unwrap();

        assert_eq!("INSERT INTO \"foo\" (\"foo\",\"baz\") VALUES ($1,DEFAULT)", sql);
    }

    #[test]
    fn join_is_inserted_positionally() {
        let joined_table = Table::from("User").left_join(
            "Post"
                .alias("p")
                .on(("p", "userId").equals(Column::from(("User", "id")))),
        );
        let q = Select::from_table(joined_table).and_from("Toto");
        let (sql, _) = Postgres::build(q).unwrap();

        assert_eq!("SELECT \"User\".*, \"Toto\".* FROM \"User\" LEFT JOIN \"Post\" AS \"p\" ON \"p\".\"userId\" = \"User\".\"id\", \"Toto\"", sql);
    }
}
