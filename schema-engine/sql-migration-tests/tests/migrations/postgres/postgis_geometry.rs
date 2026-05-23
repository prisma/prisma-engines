use indoc::indoc;
use psl::parser_database::NoExtensionTypes;
use schema_core::schema_connector::{CompositeTypeDepth, IntrospectionContext, SchemaConnector};
use sql_migration_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn create_table_with_geometry(api: TestApi) {
    let dm = indoc! {r#"
        model Location {
            id       Int      @id @default(autoincrement())
            position Geometry? @db.Geometry(Point, 4326)
        }
    "#};

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    api.schema_push_w_datasource(dm).send().assert_green();

    let connector = psl::builtin_connectors::POSTGRES;
    api.assert_schema().assert_table("Location", |table| {
        table.assert_column("position", |col| {
            col.assert_native_type("Geometry(Point,4326)", connector)
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn alter_geometry_srid(api: TestApi) {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    let schema1 = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry @db.Geometry(Point, 4326)
        }
    "#};

    api.schema_push_w_datasource(schema1).send().assert_green();

    let schema2 = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry @db.Geometry(Point, 3857)
        }
    "#};

    api.schema_push_w_datasource(schema2).send().assert_green();

    let connector = psl::builtin_connectors::POSTGRES;
    api.assert_schema().assert_table("Location", |table| {
        table.assert_column("position", |col| {
            col.assert_native_type("Geometry(Point,3857)", connector)
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn geometry_round_trip(mut api: TestApi) {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    let dm = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry? @db.Geometry(Point, 4326)
            path     Geometry? @db.Geometry(LineString, 4326)
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green();

    let schema = api.datamodel_with_provider(dm);
    let previous_schema = psl::validate_without_extensions(schema.into());
    let mut ctx = IntrospectionContext::new(
        previous_schema,
        CompositeTypeDepth::Infinite,
        None,
        std::path::PathBuf::new(),
    );
    ctx.render_config = false;

    let introspected = tok(api.connector.introspect(&ctx, &NoExtensionTypes))
        .unwrap()
        .into_single_datamodel();

    assert!(introspected.contains("@db.Geometry(Point, 4326)"));
    assert!(introspected.contains("@db.Geometry(LineString, 4326)"));
}

// SevInf #1/#4: `Geography` is a first-class PSL keyword paired with `@db.Geography(...)`.
// PostGIS persists the geodetic kind in `pg_attribute.atttypid`, so a pushed `Geography` column
// must come back as `geography(...)` and the renderer must round-trip it intact.
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn create_table_with_geography(api: TestApi) {
    let dm = indoc! {r#"
        model Place {
            id     Int        @id @default(autoincrement())
            region Geography? @db.Geography(Polygon, 4326)
        }
    "#};

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    api.schema_push_w_datasource(dm).send().assert_green();

    let connector = psl::builtin_connectors::POSTGRES;
    api.assert_schema().assert_table("Place", |table| {
        table.assert_column("region", |col| {
            col.assert_native_type("Geography(Polygon,4326)", connector)
        })
    });
}

// SevInf #1/#4 + introspection regression: Re-introspecting a `geography(...)` column must
// produce `Geography @db.Geography(...)`, not `Geometry @db.Geography(...)`. The latter is
// rejected at PSL validation time ("Native type Geography is not compatible with declared field
// type Geometry"), so a missing pairing here would silently break round-trip migrations.
#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn geography_round_trip(mut api: TestApi) {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    let dm = indoc! {r#"
        model Place {
            id       Int        @id
            location Geography? @db.Geography(Point, 4326)
            region   Geography? @db.Geography(Polygon, 4326)
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green();

    let schema = api.datamodel_with_provider(dm);
    let previous_schema = psl::validate_without_extensions(schema.into());
    let mut ctx = IntrospectionContext::new(
        previous_schema,
        CompositeTypeDepth::Infinite,
        None,
        std::path::PathBuf::new(),
    );
    ctx.render_config = false;

    let introspected = tok(api.connector.introspect(&ctx, &NoExtensionTypes))
        .unwrap()
        .into_single_datamodel();

    // The renderer must emit the `Geography` PSL keyword, NOT `Geometry`, otherwise PSL
    // validation refuses the resulting schema (see the keyword-mismatch validation fixture).
    assert!(
        introspected.contains("Geography? @db.Geography(Point, 4326)"),
        "expected `Geography? @db.Geography(Point, 4326)` in re-introspected schema, got:\n{introspected}",
    );
    assert!(
        introspected.contains("Geography? @db.Geography(Polygon, 4326)"),
        "expected `Geography? @db.Geography(Polygon, 4326)` in re-introspected schema, got:\n{introspected}",
    );
    assert!(
        !introspected.contains("Geometry @db.Geography") && !introspected.contains("Geometry? @db.Geography"),
        "re-introspected schema must NOT pair `Geometry` field type with `@db.Geography` native attribute, got:\n{introspected}",
    );
}
