use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Spatialite))]
async fn native_spatial_type_columns_feature_on(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        SELECT
            AddGeometryColumn('User', 'geometry_xy', 4326, 'GEOMETRY', 'XY', 0),
            AddGeometryColumn('User', 'geometry_xyz', 4326, 'GEOMETRY', 'XYZ', 0),
            AddGeometryColumn('User', 'point_xy', 4326, 'POINT', 'XY', 0),
            AddGeometryColumn('User', 'point_xyz', 4326, 'POINT', 'XYZ', 0),
            AddGeometryColumn('User', 'linestring_xy', 4326, 'LINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'linestring_xyz', 4326, 'LINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'polygon_xy', 4326, 'POLYGON', 'XY', 0),
            AddGeometryColumn('User', 'polygon_xyz', 4326, 'POLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'multipoint_xy', 4326, 'MULTIPOINT', 'XY', 0),
            AddGeometryColumn('User', 'multipoint_xyz', 4326, 'MULTIPOINT', 'XYZ', 0),
            AddGeometryColumn('User', 'multilinestring_xy', 4326, 'MULTILINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'multilinestring_xyz', 4326, 'MULTILINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'multipolygon_xy', 4326, 'MULTIPOLYGON', 'XY', 0),
            AddGeometryColumn('User', 'multipolygon_xyz', 4326, 'MULTIPOLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'geometrycollection_xy', 4326, 'GEOMETRYCOLLECTION', 'XY', 0),
            AddGeometryColumn('User', 'geometrycollection_xyz', 4326, 'GEOMETRYCOLLECTION', 'XYZ', 0);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model User {
          id                      Int       @id @default(autoincrement())
          geometry_xy             Geometry? @db.Geometry(Geometry, 4326)
          geometry_xyz            Geometry? @db.Geometry(GeometryZ, 4326)
          point_xy                Geometry? @db.Geometry(Point, 4326)
          point_xyz               Geometry? @db.Geometry(PointZ, 4326)
          linestring_xy           Geometry? @db.Geometry(LineString, 4326)
          linestring_xyz          Geometry? @db.Geometry(LineStringZ, 4326)
          polygon_xy              Geometry? @db.Geometry(Polygon, 4326)
          polygon_xyz             Geometry? @db.Geometry(PolygonZ, 4326)
          multipoint_xy           Geometry? @db.Geometry(MultiPoint, 4326)
          multipoint_xyz          Geometry? @db.Geometry(MultiPointZ, 4326)
          multilinestring_xy      Geometry? @db.Geometry(MultiLineString, 4326)
          multilinestring_xyz     Geometry? @db.Geometry(MultiLineStringZ, 4326)
          multipolygon_xy         Geometry? @db.Geometry(MultiPolygon, 4326)
          multipolygon_xyz        Geometry? @db.Geometry(MultiPolygonZ, 4326)
          geometrycollection_xy   Geometry? @db.Geometry(GeometryCollection, 4326)
          geometrycollection_xyz  Geometry? @db.Geometry(GeometryCollectionZ, 4326)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
