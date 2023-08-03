use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Spatialite))]
async fn native_spatial_type_columns_feature_on(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        SELECT InitSpatialMetaData();
        
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        SELECT
            AddGeometryColumn('User', 'geometry_xy', 3857, 'GEOMETRY', 'XY', 0),
            AddGeometryColumn('User', 'geometry_xyz', 3857, 'GEOMETRY', 'XYZ', 0),
            AddGeometryColumn('User', 'geometry_xym', 3857, 'GEOMETRY', 'XYM', 0),
            AddGeometryColumn('User', 'geometry_xyzm', 3857, 'GEOMETRY', 'XYZM', 0),
            AddGeometryColumn('User', 'point_xy', 3857, 'POINT', 'XY', 0),
            AddGeometryColumn('User', 'point_xyz', 3857, 'POINT', 'XYZ', 0),
            AddGeometryColumn('User', 'point_xym', 3857, 'POINT', 'XYM', 0),
            AddGeometryColumn('User', 'point_xyzm', 3857, 'POINT', 'XYZM', 0),
            AddGeometryColumn('User', 'linestring_xy', 3857, 'LINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'linestring_xyz', 3857, 'LINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'linestring_xym', 3857, 'LINESTRING', 'XYM', 0),
            AddGeometryColumn('User', 'linestring_xyzm', 3857, 'LINESTRING', 'XYZM', 0),
            AddGeometryColumn('User', 'polygon_xy', 3857, 'POLYGON', 'XY', 0),
            AddGeometryColumn('User', 'polygon_xyz', 3857, 'POLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'polygon_xym', 3857, 'POLYGON', 'XYM', 0),
            AddGeometryColumn('User', 'polygon_xyzm', 3857, 'POLYGON', 'XYZM', 0),
            AddGeometryColumn('User', 'multipoint_xy', 3857, 'MULTIPOINT', 'XY', 0),
            AddGeometryColumn('User', 'multipoint_xyz', 3857, 'MULTIPOINT', 'XYZ', 0),
            AddGeometryColumn('User', 'multipoint_xym', 3857, 'MULTIPOINT', 'XYM', 0),
            AddGeometryColumn('User', 'multipoint_xyzm', 3857, 'MULTIPOINT', 'XYZM', 0),
            AddGeometryColumn('User', 'multilinestring_xy', 3857, 'MULTILINESTRING', 'XY', 0),
            AddGeometryColumn('User', 'multilinestring_xyz', 3857, 'MULTILINESTRING', 'XYZ', 0),
            AddGeometryColumn('User', 'multilinestring_xym', 3857, 'MULTILINESTRING', 'XYM', 0),
            AddGeometryColumn('User', 'multilinestring_xyzm', 3857, 'MULTILINESTRING', 'XYZM', 0),
            AddGeometryColumn('User', 'multipolygon_xy', 3857, 'MULTIPOLYGON', 'XY', 0),
            AddGeometryColumn('User', 'multipolygon_xyz', 3857, 'MULTIPOLYGON', 'XYZ', 0),
            AddGeometryColumn('User', 'multipolygon_xym', 3857, 'MULTIPOLYGON', 'XYM', 0),
            AddGeometryColumn('User', 'multipolygon_xyzm', 3857, 'MULTIPOLYGON', 'XYZM', 0),
            AddGeometryColumn('User', 'geometrycollection_xy', 3857, 'GEOMETRYCOLLECTION', 'XY', 0),
            AddGeometryColumn('User', 'geometrycollection_xyz', 3857, 'GEOMETRYCOLLECTION', 'XYZ', 0),
            AddGeometryColumn('User', 'geometrycollection_xym', 3857, 'GEOMETRYCOLLECTION', 'XYM', 0),
            AddGeometryColumn('User', 'geometrycollection_xyzm', 3857, 'GEOMETRYCOLLECTION', 'XYZM', 0);
   "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        model User {
          id                      Int       @id @default(autoincrement())
          geometry_xy             Geometry? @db.Geometry(Geometry, 3857)
          geometry_xyz            Geometry? @db.Geometry(GeometryZ, 3857)
          geometry_xym            Geometry? @db.Geometry(GeometryM, 3857)
          geometry_xyzm           Geometry? @db.Geometry(GeometryZM, 3857)
          point_xy                Geometry? @db.Geometry(Point, 3857)
          point_xyz               Geometry? @db.Geometry(PointZ, 3857)
          point_xym               Geometry? @db.Geometry(PointM, 3857)
          point_xyzm              Geometry? @db.Geometry(PointZM, 3857)
          linestring_xy           Geometry? @db.Geometry(LineString, 3857)
          linestring_xyz          Geometry? @db.Geometry(LineStringZ, 3857)
          linestring_xym          Geometry? @db.Geometry(LineStringM, 3857)
          linestring_xyzm         Geometry? @db.Geometry(LineStringZM, 3857)
          polygon_xy              Geometry? @db.Geometry(Polygon, 3857)
          polygon_xyz             Geometry? @db.Geometry(PolygonZ, 3857)
          polygon_xym             Geometry? @db.Geometry(PolygonM, 3857)
          polygon_xyzm            Geometry? @db.Geometry(PolygonZM, 3857)
          multipoint_xy           Geometry? @db.Geometry(MultiPoint, 3857)
          multipoint_xyz          Geometry? @db.Geometry(MultiPointZ, 3857)
          multipoint_xym          Geometry? @db.Geometry(MultiPointM, 3857)
          multipoint_xyzm         Geometry? @db.Geometry(MultiPointZM, 3857)
          multilinestring_xy      Geometry? @db.Geometry(MultiLineString, 3857)
          multilinestring_xyz     Geometry? @db.Geometry(MultiLineStringZ, 3857)
          multilinestring_xym     Geometry? @db.Geometry(MultiLineStringM, 3857)
          multilinestring_xyzm    Geometry? @db.Geometry(MultiLineStringZM, 3857)
          multipolygon_xy         Geometry? @db.Geometry(MultiPolygon, 3857)
          multipolygon_xyz        Geometry? @db.Geometry(MultiPolygonZ, 3857)
          multipolygon_xym        Geometry? @db.Geometry(MultiPolygonM, 3857)
          multipolygon_xyzm       Geometry? @db.Geometry(MultiPolygonZM, 3857)
          geometrycollection_xy   Geometry? @db.Geometry(GeometryCollection, 3857)
          geometrycollection_xyz  Geometry? @db.Geometry(GeometryCollectionZ, 3857)
          geometrycollection_xym  Geometry? @db.Geometry(GeometryCollectionM, 3857)
          geometrycollection_xyzm Geometry? @db.Geometry(GeometryCollectionZM, 3857)
        }
    "#]];

    expectation.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
