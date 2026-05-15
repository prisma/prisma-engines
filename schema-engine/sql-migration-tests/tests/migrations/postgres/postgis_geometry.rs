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
            position Geometry(Point, 4326)?
        }
    "#};

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    api.schema_push_w_datasource(dm).send().assert_green();

    let connector = psl::builtin_connectors::POSTGRES;
    api.assert_schema().assert_table("Location", |table| {
        table.assert_column("position", |col| {
            col.assert_native_type("geometry(Point,4326)", connector)
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn alter_geometry_srid(api: TestApi) {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    let schema1 = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry(Point, 4326)
        }
    "#};

    api.schema_push_w_datasource(schema1).send().assert_green();

    let schema2 = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry(Point, 3857)
        }
    "#};

    api.schema_push_w_datasource(schema2).send().assert_green();

    let connector = psl::builtin_connectors::POSTGRES;
    api.assert_schema().assert_table("Location", |table| {
        table.assert_column("position", |col| {
            col.assert_native_type("geometry(Point,3857)", connector)
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn geometry_round_trip(mut api: TestApi) {
    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS postgis");

    let dm = indoc! {r#"
        model Location {
            id       Int @id
            position Geometry(Point, 4326)?
            path     Geometry(LineString, 4326)?
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

    assert!(introspected.contains("Geometry(Point, 4326)"));
    assert!(introspected.contains("Geometry(LineString, 4326)"));
}
