use crate::{
    ast::*,
    visitor::{self, Visitor},
};
use std::fmt::{self, Write};

/// A visitor to generate queries for the PostgreSQL database.
///
/// The returned parameter values implement the `ToSql` trait from postgres and
/// can be used directly with the database.
#[cfg_attr(feature = "docs", doc(cfg(feature = "postgresql")))]
pub struct Postgres<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Visitor<'a> for Postgres<'a> {
    const C_BACKTICK_OPEN: &'static str = "\"";
    const C_BACKTICK_CLOSE: &'static str = "\"";
    const C_WILDCARD: &'static str = "%";

    #[tracing::instrument(name = "render_sql", skip(query))]
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
        write!(&mut self.query, "{}", s)?;
        Ok(())
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn parameter_substitution(&mut self) -> visitor::Result {
        self.write("$")?;
        self.write(self.parameters.len())
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
        let res = match value {
            Value::Integer(i) => i.map(|i| self.write(i)),
            Value::Text(t) => t.map(|t| self.write(format!("'{}'", t))),
            Value::Enum(e) => e.map(|e| self.write(e)),
            Value::Bytes(b) => b.map(|b| self.write(format!("E'{}'", hex::encode(b)))),
            Value::Boolean(b) => b.map(|b| self.write(b)),
            Value::Xml(cow) => cow.map(|cow| self.write(format!("'{}'", cow))),
            Value::Char(c) => c.map(|c| self.write(format!("'{}'", c))),
            Value::Float(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f32::INFINITY => self.write("'Infinity'"),
                f if f == f32::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{:?}", v)),
            }),
            Value::Double(d) => d.map(|f| match f {
                f if f.is_nan() => self.write("'NaN'"),
                f if f == f64::INFINITY => self.write("'Infinity'"),
                f if f == f64::NEG_INFINITY => self.write("'-Infinity"),
                v => self.write(format!("{:?}", v)),
            }),
            Value::Array(ary) => ary.map(|ary| {
                self.surround_with("'{", "}'", |ref mut s| {
                    let len = ary.len();

                    for (i, item) in ary.into_iter().enumerate() {
                        s.write(item)?;

                        if i < len - 1 {
                            s.write(",")?;
                        }
                    }

                    Ok(())
                })
            }),
            #[cfg(feature = "json")]
            Value::Json(j) => j.map(|j| self.write(format!("'{}'", serde_json::to_string(&j).unwrap()))),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(r) => r.map(|r| self.write(r)),
            #[cfg(feature = "uuid")]
            Value::Uuid(uuid) => uuid.map(|uuid| self.write(format!("'{}'", uuid.to_hyphenated().to_string()))),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.map(|dt| self.write(format!("'{}'", dt.to_rfc3339(),))),
            #[cfg(feature = "chrono")]
            Value::Date(date) => date.map(|date| self.write(format!("'{}'", date))),
            #[cfg(feature = "chrono")]
            Value::Time(time) => time.map(|time| self.write(format!("'{}'", time))),
        };

        match res {
            Some(res) => res,
            None => self.write("null"),
        }
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> visitor::Result {
        self.write("INSERT ")?;

        if let Some(table) = insert.table {
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

        if let Some(OnConflict::DoNothing) = insert.on_conflict {
            self.write(" ON CONFLICT DO NOTHING")?;
        };

        if let Some(returning) = insert.returning {
            if !returning.is_empty() {
                let values = returning.into_iter().map(|r| r.into()).collect();
                self.write(" RETURNING ")?;
                self.visit_columns(values)?;
            }
        };

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
            #[cfg(feature = "json")]
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            #[cfg(feature = "json")]
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
            #[cfg(feature = "json")]
            _ if left.is_json_value() => "::jsonb",
            _ if left.is_xml_value() => "::text",
            _ => "",
        };

        let left_cast = match right {
            #[cfg(feature = "json")]
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

    fn default_params<'a>(mut additional: Vec<Value<'a>>) -> Vec<Value<'a>> {
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
    #[cfg(feature = "postgres")]
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
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1 OFFSET $2", vec![10, 2]);
        let query = Select::from_table("users").limit(10).offset(2);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_offset_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" OFFSET $1", vec![10]);
        let query = Select::from_table("users").offset(10);
        let (sql, params) = Postgres::build(query).unwrap();

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_limit_and_offset_when_only_limit_is_set() {
        let expected = expected_values("SELECT \"users\".* FROM \"users\" LIMIT $1", vec![10]);
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

    #[cfg(feature = "json")]
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

    #[cfg(feature = "json")]
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

    #[cfg(feature = "json")]
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

    #[cfg(feature = "json")]
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
        let (sql, params) = Postgres::build(Select::default().value(Value::Text(None).raw())).unwrap();
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
    #[cfg(feature = "json")]
    fn test_raw_json() {
        let (sql, params) =
            Postgres::build(Select::default().value(serde_json::json!({ "foo": "bar" }).raw())).unwrap();
        assert_eq!("SELECT '{\"foo\":\"bar\"}'", sql);
        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn test_raw_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let (sql, params) = Postgres::build(Select::default().value(uuid.raw())).unwrap();

        assert_eq!(format!("SELECT '{}'", uuid.to_hyphenated().to_string()), sql);

        assert!(params.is_empty());
    }

    #[test]
    #[cfg(feature = "chrono")]
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
