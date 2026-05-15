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
    assert!(schema.contains("position Geometry(Point, 4326)?"));
    assert!(schema.contains("path") && schema.contains("Geometry(LineString)"));
    assert!(schema.contains("area Geometry(Polygon, 3857)"));

    Ok(())
}
