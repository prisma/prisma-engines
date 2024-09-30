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
            AddGeometryColumn('User', 'geometry_xym', 4326, 'GEOMETRY', 'XYM', 0),
            AddGeometryColumn('User', 'geometry_xyzm', 4326, 'GEOMETRY', 'XYZM', 0),
            AddGeometryColumn('User', 'point_xy', 4326, 'POINT', 'XY', 0),
            AddGeometryColumn('User', 'point_xyz', 4326, 'POINT', 'XYZ', 0),
            AddGeometryColumn('User', 'point_xym', 4326, 'POINT', 'XYM', 0),
            AddGeometryColumn('User', 'point_xyzm', 4326, 'POINT', 'XYZM', 0),
            AddGeometryColumn('User', 'linestring_xy', 4326, 'LINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'linestring_xyz', 4326, 'LINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'linestring_xym', 4326, 'LINESTRING', 'XYM', 0),
            AddGeometryColumn('User', 'linestring_xyzm', 4326, 'LINESTRING', 'XYZM', 0),
            AddGeometryColumn('User', 'polygon_xy', 4326, 'POLYGON', 'XY', 0),
            AddGeometryColumn('User', 'polygon_xyz', 4326, 'POLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'polygon_xym', 4326, 'POLYGON', 'XYM', 0),
            AddGeometryColumn('User', 'polygon_xyzm', 4326, 'POLYGON', 'XYZM', 0),
            AddGeometryColumn('User', 'multipoint_xy', 4326, 'MULTIPOINT', 'XY', 0),
            AddGeometryColumn('User', 'multipoint_xyz', 4326, 'MULTIPOINT', 'XYZ', 0),
            AddGeometryColumn('User', 'multipoint_xym', 4326, 'MULTIPOINT', 'XYM', 0),
            AddGeometryColumn('User', 'multipoint_xyzm', 4326, 'MULTIPOINT', 'XYZM', 0),
            AddGeometryColumn('User', 'multilinestring_xy', 4326, 'MULTILINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'multilinestring_xyz', 4326, 'MULTILINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'multilinestring_xym', 4326, 'MULTILINESTRING', 'XYM', 0),
            AddGeometryColumn('User', 'multilinestring_xyzm', 4326, 'MULTILINESTRING', 'XYZM', 0),
            AddGeometryColumn('User', 'multipolygon_xy', 4326, 'MULTIPOLYGON', 'XY', 0),
            AddGeometryColumn('User', 'multipolygon_xyz', 4326, 'MULTIPOLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'multipolygon_xym', 4326, 'MULTIPOLYGON', 'XYM', 0),
            AddGeometryColumn('User', 'multipolygon_xyzm', 4326, 'MULTIPOLYGON', 'XYZM', 0),
            AddGeometryColumn('User', 'geometrycollection_xy', 4326, 'GEOMETRYCOLLECTION', 'XY', 0),
            AddGeometryColumn('User', 'geometrycollection_xyz', 4326, 'GEOMETRYCOLLECTION', 'XYZ', 0),
            AddGeometryColumn('User', 'geometrycollection_xym', 4326, 'GEOMETRYCOLLECTION', 'XYM', 0),
            AddGeometryColumn('User', 'geometrycollection_xyzm', 4326, 'GEOMETRYCOLLECTION', 'XYZM', 0);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model User {
          id                      Int       @id @default(autoincrement())
          geometry_xy             Geometry? @db.Geometry(Geometry, 4326)
          geometry_xyz            Geometry? @db.Geometry(GeometryZ, 4326)
          geometry_xym            Geometry? @db.Geometry(GeometryM, 4326)
          geometry_xyzm           Geometry? @db.Geometry(GeometryZM, 4326)
          point_xy                Geometry? @db.Geometry(Point, 4326)
          point_xyz               Geometry? @db.Geometry(PointZ, 4326)
          point_xym               Geometry? @db.Geometry(PointM, 4326)
          point_xyzm              Geometry? @db.Geometry(PointZM, 4326)
          linestring_xy           Geometry? @db.Geometry(LineString, 4326)
          linestring_xyz          Geometry? @db.Geometry(LineStringZ, 4326)
          linestring_xym          Geometry? @db.Geometry(LineStringM, 4326)
          linestring_xyzm         Geometry? @db.Geometry(LineStringZM, 4326)
          polygon_xy              Geometry? @db.Geometry(Polygon, 4326)
          polygon_xyz             Geometry? @db.Geometry(PolygonZ, 4326)
          polygon_xym             Geometry? @db.Geometry(PolygonM, 4326)
          polygon_xyzm            Geometry? @db.Geometry(PolygonZM, 4326)
          multipoint_xy           Geometry? @db.Geometry(MultiPoint, 4326)
          multipoint_xyz          Geometry? @db.Geometry(MultiPointZ, 4326)
          multipoint_xym          Geometry? @db.Geometry(MultiPointM, 4326)
          multipoint_xyzm         Geometry? @db.Geometry(MultiPointZM, 4326)
          multilinestring_xy      Geometry? @db.Geometry(MultiLineString, 4326)
          multilinestring_xyz     Geometry? @db.Geometry(MultiLineStringZ, 4326)
          multilinestring_xym     Geometry? @db.Geometry(MultiLineStringM, 4326)
          multilinestring_xyzm    Geometry? @db.Geometry(MultiLineStringZM, 4326)
          multipolygon_xy         Geometry? @db.Geometry(MultiPolygon, 4326)
          multipolygon_xyz        Geometry? @db.Geometry(MultiPolygonZ, 4326)
          multipolygon_xym        Geometry? @db.Geometry(MultiPolygonM, 4326)
          multipolygon_xyzm       Geometry? @db.Geometry(MultiPolygonZM, 4326)
          geometrycollection_xy   Geometry? @db.Geometry(GeometryCollection, 4326)
          geometrycollection_xyz  Geometry? @db.Geometry(GeometryCollectionZ, 4326)
          geometrycollection_xym  Geometry? @db.Geometry(GeometryCollectionM, 4326)
          geometrycollection_xyzm Geometry? @db.Geometry(GeometryCollectionZM, 4326)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
