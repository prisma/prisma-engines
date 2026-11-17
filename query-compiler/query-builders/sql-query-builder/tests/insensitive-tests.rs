use quaint::{
    Value, ValueType,
    ast::Select,
    connector::{ExternalConnectionInfo, SqlFamily},
    prelude::ConnectionInfo,
    visitor::{KingbaseOracle, Postgres, Visitor},
};
use query_structure::{
    ConditionListValue, ConditionValue, Filter, PrismaValue, QueryMode, ScalarCondition, ScalarFieldRef, ScalarFilter,
    ScalarProjection,
};
use sql_query_builder::{Context, FilterBuilder};
use std::sync::Arc;

fn fields(family: SqlFamily) -> (ScalarFieldRef, ScalarFieldRef) {
    let provider = match family {
        SqlFamily::KingbaseOracle => "kingbase-oracle",
        SqlFamily::Postgres => "postgresql",
        _ => unreachable!(),
    };
    let schema = psl::parse_schema_without_extensions(
        r#"
        datasource db {
            provider = "kingbase-oracle"
        }

        model User {
            id     Int     @id
            name   String?
            needle String?
        }
        "#
        .replace("kingbase-oracle", provider),
    )
    .unwrap();
    let model = query_structure::convert(Arc::new(schema)).find_model("User").unwrap();
    (
        model.fields().find_from_scalar("name").unwrap(),
        model.fields().find_from_scalar("needle").unwrap(),
    )
}

fn render(family: SqlFamily, condition: ScalarCondition, mode: QueryMode) -> (String, Vec<Value<'static>>) {
    let (field, _) = fields(family);
    let connection = ConnectionInfo::External(ExternalConnectionInfo::new(family, None, None, false));
    let ctx = Context::new(&connection, None);
    let filter = ScalarFilter {
        projection: ScalarProjection::Single(field),
        condition,
        mode,
    };
    let condition = FilterBuilder::without_top_level_joins().visit_filter(Filter::Scalar(filter), &ctx);
    let query = Select::from_table("User").column("id").so_that(condition);

    match family {
        SqlFamily::KingbaseOracle => KingbaseOracle::build(query).unwrap(),
        SqlFamily::Postgres => Postgres::build(query).unwrap(),
        _ => unreachable!(),
    }
}

type PatternCondition = fn(ConditionValue) -> ScalarCondition;

fn pattern_cases() -> [(PatternCondition, &'static str, &'static str); 8] {
    [
        (ScalarCondition::Equals, "LIKE", "AlIcE"),
        (ScalarCondition::NotEquals, "NOT LIKE", "AlIcE"),
        (ScalarCondition::Contains, "LIKE", "%AlIcE%"),
        (ScalarCondition::NotContains, "NOT LIKE", "%AlIcE%"),
        (ScalarCondition::StartsWith, "LIKE", "AlIcE%"),
        (ScalarCondition::NotStartsWith, "NOT LIKE", "AlIcE%"),
        (ScalarCondition::EndsWith, "LIKE", "%AlIcE"),
        (ScalarCondition::NotEndsWith, "NOT LIKE", "%AlIcE"),
    ]
}

#[test]
fn oracle_insensitive_patterns_fold_both_operands() {
    for (condition, operator, pattern) in pattern_cases() {
        let (sql, params) = render(
            SqlFamily::KingbaseOracle,
            condition(PrismaValue::String("AlIcE".into()).into()),
            QueryMode::Insensitive,
        );
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE LOWER("User"."name") {operator} LOWER($1)"#)
        );
        assert_eq!(params, vec![Value::text(pattern)]);
    }
}

#[test]
fn oracle_insensitive_field_references_fold_the_complete_pattern() {
    let (_, needle) = fields(SqlFamily::KingbaseOracle);
    let cases = [
        (
            ScalarCondition::Equals as PatternCondition,
            "LIKE",
            r#""User"."needle""#,
            vec![],
        ),
        (ScalarCondition::NotEquals, "NOT LIKE", r#""User"."needle""#, vec![]),
        (
            ScalarCondition::Contains,
            "LIKE",
            r#"($1 || "User"."needle" || $2)"#,
            vec!["%", "%"],
        ),
        (
            ScalarCondition::NotContains,
            "NOT LIKE",
            r#"($1 || "User"."needle" || $2)"#,
            vec!["%", "%"],
        ),
        (
            ScalarCondition::StartsWith,
            "LIKE",
            r#"("User"."needle" || $1)"#,
            vec!["%"],
        ),
        (
            ScalarCondition::NotStartsWith,
            "NOT LIKE",
            r#"("User"."needle" || $1)"#,
            vec!["%"],
        ),
        (
            ScalarCondition::EndsWith,
            "LIKE",
            r#"($1 || "User"."needle")"#,
            vec!["%"],
        ),
        (
            ScalarCondition::NotEndsWith,
            "NOT LIKE",
            r#"($1 || "User"."needle")"#,
            vec!["%"],
        ),
    ];

    for (condition, operator, pattern, values) in cases {
        let (sql, params) = render(
            SqlFamily::KingbaseOracle,
            condition(ConditionValue::FieldRef(needle.clone())),
            QueryMode::Insensitive,
        );
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE LOWER("User"."name") {operator} LOWER({pattern})"#)
        );
        assert_eq!(params, values.into_iter().map(Value::text).collect::<Vec<_>>());
    }
}

#[test]
fn postgres_insensitive_patterns_keep_ilike() {
    for (condition, operator, pattern) in pattern_cases() {
        let (sql, params) = render(
            SqlFamily::Postgres,
            condition(PrismaValue::String("AlIcE".into()).into()),
            QueryMode::Insensitive,
        );
        let operator = if operator == "LIKE" { "ILIKE" } else { "NOT ILIKE" };
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE "User"."name" {operator} $1"#)
        );
        assert_eq!(params, vec![Value::text(pattern)]);
    }
}

#[test]
fn postgres_insensitive_field_references_keep_ilike() {
    let (_, needle) = fields(SqlFamily::Postgres);
    for (condition, operator, _) in pattern_cases() {
        let (sql, _) = render(
            SqlFamily::Postgres,
            condition(ConditionValue::FieldRef(needle.clone())),
            QueryMode::Insensitive,
        );
        let operator = if operator == "LIKE" { "ILIKE" } else { "NOT ILIKE" };
        assert!(sql.contains(&format!(r#""User"."name" {operator} "#)), "{sql}");
        assert!(!sql.contains("LOWER("), "{sql}");
    }
}

#[test]
fn oracle_null_filters_keep_is_null() {
    for (condition, operator) in [
        (ScalarCondition::Equals as PatternCondition, "IS NULL"),
        (ScalarCondition::NotEquals, "IS NOT NULL"),
    ] {
        let (sql, params) = render(
            SqlFamily::KingbaseOracle,
            condition(PrismaValue::Null.into()),
            QueryMode::Insensitive,
        );
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE "User"."name" {operator}"#)
        );
        assert!(params.is_empty());
    }
}

#[test]
fn oracle_insensitive_placeholders_stay_bound_inside_patterns() {
    for (condition, operator, pattern) in [
        (ScalarCondition::Equals as PatternCondition, "LIKE", "$1"),
        (ScalarCondition::NotEquals, "NOT LIKE", "$1"),
        (ScalarCondition::Contains, "LIKE", "('%' || $1 || '%')"),
        (ScalarCondition::NotContains, "NOT LIKE", "('%' || $1 || '%')"),
        (ScalarCondition::StartsWith, "LIKE", "($1 || '%')"),
        (ScalarCondition::NotStartsWith, "NOT LIKE", "($1 || '%')"),
        (ScalarCondition::EndsWith, "LIKE", "('%' || $1)"),
        (ScalarCondition::NotEndsWith, "NOT LIKE", "('%' || $1)"),
    ] {
        let value = PrismaValue::placeholder("needle", prisma_value::PrismaValueType::String);
        let (sql, params) = render(
            SqlFamily::KingbaseOracle,
            condition(value.into()),
            QueryMode::Insensitive,
        );
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE LOWER("User"."name") {operator} LOWER({pattern})"#)
        );
        assert_eq!(params.len(), 1);
        assert!(matches!(params[0].typed, ValueType::Opaque(_)));
    }
}

#[test]
fn oracle_default_mode_keeps_case_sensitive_filter() {
    let (sql, params) = render(
        SqlFamily::KingbaseOracle,
        ScalarCondition::Contains(PrismaValue::String("AlIcE".into()).into()),
        QueryMode::Default,
    );
    assert_eq!(sql, r#"SELECT "id" FROM "User" WHERE "User"."name" LIKE $1"#);
    assert_eq!(params, vec![Value::text("%AlIcE%")]);
}

#[test]
fn oracle_membership_and_ordering_keep_case_folding() {
    let values = vec![PrismaValue::String("Alice".into()), PrismaValue::String("BOB".into())];
    for (condition, expected) in [
        (
            ScalarCondition::In(ConditionListValue::List(values.clone())),
            "IN (LOWER($1),LOWER($2))",
        ),
        (
            ScalarCondition::NotIn(ConditionListValue::List(values)),
            "NOT IN (LOWER($1),LOWER($2))",
        ),
        (
            ScalarCondition::LessThan(PrismaValue::String("Bob".into()).into()),
            "< LOWER($1)",
        ),
        (
            ScalarCondition::GreaterThanOrEquals(PrismaValue::String("Bob".into()).into()),
            ">= LOWER($1)",
        ),
    ] {
        let (sql, _) = render(SqlFamily::KingbaseOracle, condition, QueryMode::Insensitive);
        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "User" WHERE LOWER("User"."name") {expected}"#)
        );
    }
}
