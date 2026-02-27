use crate::visitor::query_writer::QueryWriter;
use crate::{
    ast::*,
    error::{Error, ErrorKind},
    visitor::{self, Visitor},
};
use itertools::Itertools;
use query_template::{PlaceholderFormat, QueryTemplate};
use std::borrow::Cow;
use std::{fmt, ops::Deref};

/// A visitor to generate queries for the PostgreSQL database.
///
/// The returned parameter values implement the `ToSql` trait from postgres and
/// can be used directly with the database.
pub struct Postgres<'a> {
    query_template: QueryTemplate<Value<'a>>,
}

impl<'a> Postgres<'a> {
    /// Expression that evaluates to the current PostgreSQL version.
    pub const fn version_expr() -> &'static str {
        "version()"
    }

    fn visit_json_build_obj_expr(&mut self, expr: Expression<'a>) -> crate::Result<()> {
        match expr.kind() {
            ExpressionKind::Column(col) => match (col.type_family.as_ref(), col.native_type.as_deref()) {
                (Some(TypeFamily::Decimal(_)), Some("MONEY")) => {
                    self.visit_expression(expr)?;
                    self.write("::numeric")?;

                    Ok(())
                }
                // Cast BigInt to text to preserve precision when parsed by JavaScript.
                // JavaScript's JSON.parse loses precision for integers > 2^53-1.
                (Some(TypeFamily::Int), Some("BIGINT" | "INT8")) => {
                    self.visit_expression(expr)?;
                    self.write("::text")?;

                    Ok(())
                }
                _ => self.visit_expression(expr),
            },
            _ => self.visit_expression(expr),
        }
    }

    fn visit_returning(&mut self, returning: Option<Vec<Column<'a>>>) -> visitor::Result {
        if let Some(returning) = returning
            && !returning.is_empty()
        {
            let values = returning.into_iter().map(|r| r.into()).collect();
            self.write(" RETURNING ")?;
            self.visit_columns(values)?;
        }
        Ok(())
    }
}

impl<'a> Visitor<'a> for Postgres<'a> {
    const C_BACKTICK_OPEN: &'static str = "\"";
    const C_BACKTICK_CLOSE: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    fn build_template<Q>(query: Q) -> crate::Result<QueryTemplate<Value<'a>>>
    where
        Q: Into<Query<'a>>,
    {
        let mut this = Postgres {
            query_template: QueryTemplate::new(PlaceholderFormat {
                prefix: "$",
                has_numbering: true,
            }),
        };

        Postgres::visit_query(&mut this, query.into())?;

        Ok(this.query_template)
    }

    fn write(&mut self, value: impl fmt::Display) -> visitor::Result {
        self.query_template.write_string_chunk(value.to_string());
        Ok(())
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
        let cast_target = get_column_cast_target(&column);

        match column.table {
            Some(table) => {
                self.visit_table(table, false)?;
                self.write(".")?;
                self.delimited_identifiers(&[&*column.name])?;
            }
            _ => self.delimited_identifiers(&[&*column.name])?,
        };

        if let Some(cast) = cast_target {
            self.write("::")?;
            self.write(cast)?;
            if column.is_list {
                self.write("[]")?;
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
        self.write("INSERT ")?;

        if let Some(table) = insert.table.clone() {
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
                    self.visit_column(c.name.into_owned().into())?;

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

        self.visit_returning(insert.returning)?;

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

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract(&mut self, json_extract: JsonExtract<'a>) -> visitor::Result {
        match json_extract.path {
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

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
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

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_last_array_item(&mut self, extract: JsonExtractLastArrayElem<'a>) -> visitor::Result {
        self.write("(")?;
        self.visit_expression(*extract.expr)?;
        self.write("->-1")?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
    fn visit_json_extract_first_array_item(&mut self, extract: JsonExtractFirstArrayElem<'a>) -> visitor::Result {
        self.write("(")?;
        self.visit_expression(*extract.expr)?;
        self.write("->0")?;
        self.write(")")?;

        Ok(())
    }

    #[cfg(any(feature = "postgresql", feature = "mysql", feature = "sqlite"))]
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

    #[cfg(feature = "postgresql")]
    fn visit_json_array_agg(&mut self, array_agg: JsonArrayAgg<'a>) -> visitor::Result {
        self.write("JSONB_AGG")?;
        self.surround_with("(", ")", |s| s.visit_expression(*array_agg.expr))?;

        Ok(())
    }

    #[cfg(feature = "postgresql")]
    fn visit_json_build_object(&mut self, build_obj: JsonBuildObject<'a>) -> visitor::Result {
        // Functions in PostgreSQL can only accept up to 100 arguments, which means that we can't
        // build an object with more than 50 fields using `JSON_BUILD_OBJECT`. To work around
        // that, we chunk the fields into subsets of 50 fields or less, build one or more JSONB
        // objects using one or more `JSONB_BUILD_OBJECT` invocations, and merge them together
        // using the `||` operator (which is not possible with plain JSON).
        //
        // See <https://github.com/prisma/prisma/issues/22298>.
        //
        // Another alternative that was considered for the specific use case of loading relations
        // in Query Engine was using `ROW_TO_JSON` but it turned out to not be a suitable
        // replacement for several reasons, the main one being the limit of the length of field
        // names (63 characters).
        const MAX_FIELDS: usize = 50;
        let num_chunks = build_obj.exprs.len().div_ceil(MAX_FIELDS);

        for (i, chunk) in build_obj.exprs.into_iter().chunks(MAX_FIELDS).into_iter().enumerate() {
            let mut chunk = chunk.peekable();

            self.write("JSONB_BUILD_OBJECT")?;

            self.surround_with("(", ")", |s| {
                while let Some((name, expr)) = chunk.next() {
                    s.visit_raw_value(Value::text(name))?;
                    s.write(", ")?;
                    s.visit_json_build_obj_expr(expr)?;
                    if chunk.peek().is_some() {
                        s.write(", ")?;
                    }
                }

                Ok(())
            })?;

            if i < num_chunks - 1 {
                self.write(" || ")?;
            }
        }

        Ok(())
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

    fn visit_matches(&mut self, left: Expression<'a>, right: Expression<'a>, not: bool) -> visitor::Result {
        if not {
            self.write("(NOT ")?;
        }

        self.visit_expression(left)?;
        self.write(" @@ ")?;
        self.surround_with("to_tsquery(", ")", |s| s.visit_expression(right))?;

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
        self.surround_with("to_tsquery(", ")", |s| s.visit_expression(query))?;
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

    fn visit_min(&mut self, min: Minimum<'a>) -> visitor::Result {
        // If the inner column is a selected enum, then we cast the result of MIN(enum)::text instead of casting the inner enum column, which changes the behavior of MIN.
        let cast_target = get_column_cast_target(&min.column);

        self.write("MIN")?;
        self.surround_with("(", ")", |ref mut s| s.visit_column(min.column.set_is_selected(false)))?;

        if let Some(cast_target) = cast_target {
            self.write("::")?;
            self.write(cast_target)?;
        }

        Ok(())
    }

    fn visit_max(&mut self, max: Maximum<'a>) -> visitor::Result {
        // If the inner column is a selected enum, then we cast the result of MAX(enum)::text instead of casting the inner enum column, which changes the behavior of MAX.
        let cast_target = get_column_cast_target(&max.column);

        self.write("MAX")?;
        self.surround_with("(", ")", |ref mut s| s.visit_column(max.column.set_is_selected(false)))?;

        if let Some(cast_target) = cast_target {
            self.write("::")?;
            self.write(cast_target)?;
        }

        Ok(())
    }

    fn visit_delete(&mut self, delete: Delete<'a>) -> visitor::Result {
        self.write("DELETE FROM ")?;
        self.visit_table(delete.table, true)?;

        if let Some(conditions) = delete.conditions {
            self.write(" WHERE ")?;
            self.visit_conditions(conditions)?;
        }

        self.visit_returning(delete.returning)?;

        if let Some(comment) = delete.comment {
            self.write(" ")?;
            self.visit_comment(comment)?;
        }

        Ok(())
    }
}

fn get_column_cast_target(column: &Column<'_>) -> Option<&'static str> {
    if !column.is_selected {
        return None;
    }

    if column.is_enum {
        Some("text")
    } else if column.native_type.as_deref() == Some("MONEY") || column.native_type.as_deref() == Some("MONEY[]") {
        Some("numeric")
    } else {
        None
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
    fn test_distinct_on() {
        let expected_sql = "SELECT DISTINCT ON (\"bar\", \"foo\") \"bar\" FROM \"test\"";
        let query = Select::from_table("test").column(Column::new("bar")).distinct_on(vec![
            Expression::from(Column::from("bar")),
            Expression::from(Column::from("foo")),
        ]);

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

        assert_eq!(
            "SELECT \"User\".*, \"Toto\".* FROM \"User\" LEFT JOIN \"Post\" AS \"p\" ON \"p\".\"userId\" = \"User\".\"id\", \"Toto\"",
            sql
        );
    }

    #[test]
    fn enum_cast_text_in_min_max_should_be_outside() {
        let enum_col = Column::from("enum").set_is_enum(true).set_is_selected(true);
        let q = Select::from_table("User")
            .value(min(enum_col.clone()))
            .value(max(enum_col));
        let (sql, _) = Postgres::build(q).unwrap();

        assert_eq!("SELECT MIN(\"enum\")::text, MAX(\"enum\")::text FROM \"User\"", sql);
    }

    mod test_json_build_object {
        use super::*;

        #[test]
        fn simple() {
            let build_json = build_json_object(3);
            let query = Select::default().value(build_json);
            let (sql, _) = Postgres::build(query).unwrap();

            assert_eq!("SELECT JSONB_BUILD_OBJECT('f1', $1, 'f2', $2, 'f3', $3)", sql);
        }

        #[test]
        fn chunked() {
            let build_json = build_json_object(110);
            let query = Select::default().value(build_json);
            let (sql, _) = Postgres::build(query).unwrap();

            assert_eq!(
                concat!(
                    "SELECT JSONB_BUILD_OBJECT('f1', $1, 'f2', $2, 'f3', $3, 'f4', $4, 'f5', $5, 'f6', $6, 'f7', $7, 'f8', $8, 'f9', $9, 'f10', $10, 'f11', $11, 'f12', $12, 'f13', $13, 'f14', $14, 'f15', $15, 'f16', $16, 'f17', $17, 'f18', $18, 'f19', $19, 'f20', $20, 'f21', $21, 'f22', $22, 'f23', $23, 'f24', $24, 'f25', $25, 'f26', $26, 'f27', $27, 'f28', $28, 'f29', $29, 'f30', $30, 'f31', $31, 'f32', $32, 'f33', $33, 'f34', $34, 'f35', $35, 'f36', $36, 'f37', $37, 'f38', $38, 'f39', $39, 'f40', $40, 'f41', $41, 'f42', $42, 'f43', $43, 'f44', $44, 'f45', $45, 'f46', $46, 'f47', $47, 'f48', $48, 'f49', $49, 'f50', $50)",
                    " || JSONB_BUILD_OBJECT('f51', $51, 'f52', $52, 'f53', $53, 'f54', $54, 'f55', $55, 'f56', $56, 'f57', $57, 'f58', $58, 'f59', $59, 'f60', $60, 'f61', $61, 'f62', $62, 'f63', $63, 'f64', $64, 'f65', $65, 'f66', $66, 'f67', $67, 'f68', $68, 'f69', $69, 'f70', $70, 'f71', $71, 'f72', $72, 'f73', $73, 'f74', $74, 'f75', $75, 'f76', $76, 'f77', $77, 'f78', $78, 'f79', $79, 'f80', $80, 'f81', $81, 'f82', $82, 'f83', $83, 'f84', $84, 'f85', $85, 'f86', $86, 'f87', $87, 'f88', $88, 'f89', $89, 'f90', $90, 'f91', $91, 'f92', $92, 'f93', $93, 'f94', $94, 'f95', $95, 'f96', $96, 'f97', $97, 'f98', $98, 'f99', $99, 'f100', $100)",
                    " || JSONB_BUILD_OBJECT('f101', $101, 'f102', $102, 'f103', $103, 'f104', $104, 'f105', $105, 'f106', $106, 'f107', $107, 'f108', $108, 'f109', $109, 'f110', $110)"
                ),
                sql
            );
        }

        #[test]
        fn money() {
            let build_json = json_build_object(vec![(
                "money".into(),
                Column::from("money")
                    .native_column_type(Some("money"))
                    .type_family(TypeFamily::Decimal(None))
                    .into(),
            )]);
            let query = Select::default().value(build_json);
            let (sql, _) = Postgres::build(query).unwrap();

            assert_eq!(sql, "SELECT JSONB_BUILD_OBJECT('money', \"money\"::numeric)");
        }

        #[test]
        fn bigint() {
            let build_json = json_build_object(vec![(
                "id".into(),
                Column::from("id")
                    .native_column_type(Some("BigInt"))
                    .type_family(TypeFamily::Int)
                    .into(),
            )]);
            let query = Select::default().value(build_json);
            let (sql, _) = Postgres::build(query).unwrap();

            assert_eq!(sql, "SELECT JSONB_BUILD_OBJECT('id', \"id\"::text)");
        }

        #[test]
        fn int8() {
            let build_json = json_build_object(vec![(
                "id".into(),
                Column::from("id")
                    .native_column_type(Some("INT8"))
                    .type_family(TypeFamily::Int)
                    .into(),
            )]);
            let query = Select::default().value(build_json);
            let (sql, _) = Postgres::build(query).unwrap();

            assert_eq!(sql, "SELECT JSONB_BUILD_OBJECT('id', \"id\"::text)");
        }
        fn build_json_object(num_fields: u32) -> JsonBuildObject<'static> {
            let fields = (1..=num_fields)
                .map(|i| (format!("f{i}").into(), Expression::from(i as i64)))
                .collect();

            JsonBuildObject { exprs: fields }
        }
    }
}
