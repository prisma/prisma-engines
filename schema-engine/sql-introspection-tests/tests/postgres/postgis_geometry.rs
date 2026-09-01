use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

// PostGIS extension plus a table with multiple `geometry` columns (typmod / SRID variants).
#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn introspect_geometry_columns(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis").await;
    api.raw_cmd(indoc! {r#"
        CREATE TABLE locations (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            position geometry(Point, 4326),
            path geometry(LineString),
            area geometry(Polygon, 3857) NOT NULL
        );
    "#})
        .await;

    let schema = api.introspect().await?;

    assert!(schema.contains("extensions = [postgis"));
    assert!(schema.contains("model locations"));
    // The introspector pretty-prints columns with alignment padding (`name  Type   @attr`),
    // so we normalize whitespace before the substring check instead of pinning a single layout.
    let normalized: String = schema.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(normalized.contains("position Geometry?") && normalized.contains("@db.Geometry(Point, 4326)"));
    assert!(normalized.contains("path Geometry?") && normalized.contains("@db.Geometry(LineString)"));
    assert!(normalized.contains("area Geometry") && normalized.contains("@db.Geometry(Polygon, 3857)"));

    Ok(())
}

// SevInf #1/#4: PostGIS `geography` columns must surface as `Geography @db.Geography(...)` in the
// re-introspected schema. Until this audit, the introspection layer collapsed both spatial kinds
// into the `Geometry` PSL keyword, which then collided with the `@db.Geography` native attribute
// validator ("Native type Geography is not compatible with declared field type Geometry"). This
// regression test pins the planar/geodetic split end-to-end against a live PostGIS catalog.
#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn introspect_geography_columns(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis").await;
    api.raw_cmd(indoc! {r#"
        CREATE TABLE places (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            location geography(Point, 4326),
            region geography(Polygon, 4326) NOT NULL,
            footprint geography
        );
    "#})
        .await;

    let schema = api.introspect().await?;

    assert!(schema.contains("extensions = [postgis"));
    assert!(schema.contains("model places"));

    // The renderer aligns column declarations with padding (`name  Type   @attr`); normalize
    // whitespace so substring checks pin semantics, not formatting.
    let normalized: String = schema.split_whitespace().collect::<Vec<_>>().join(" ");

    // Field type MUST be `Geography` (not `Geometry`) so the native attribute pairing validates.
    assert!(
        normalized.contains("location Geography?") && normalized.contains("@db.Geography(Point, 4326)"),
        "expected `location Geography? ... @db.Geography(Point, 4326)`, got:\n{schema}",
    );
    assert!(
        normalized.contains("region Geography") && normalized.contains("@db.Geography(Polygon, 4326)"),
        "expected `region Geography ... @db.Geography(Polygon, 4326)`, got:\n{schema}",
    );
    // Untyped `geography` (no typmod) keeps the bare native form without subtype/SRID args.
    assert!(
        normalized.contains("footprint Geography?"),
        "expected `footprint Geography?`, got:\n{schema}",
    );

    // The introspector must NEVER emit `Geometry @db.Geography(...)` — that combo is rejected by
    // PSL validation and used to be the silent failure mode for `geography` columns.
    assert!(
        !normalized.contains("Geometry @db.Geography") && !normalized.contains("Geometry? @db.Geography"),
        "introspected schema must not pair `Geometry` keyword with `@db.Geography`, got:\n{schema}",
    );

    Ok(())
}
