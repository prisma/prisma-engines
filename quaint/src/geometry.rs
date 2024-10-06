pub fn get_geometry_srid(geom: &geojson::Geometry) -> Option<i32> {
    geom.foreign_members
        .as_ref()?
        .get("crs")?
        .as_object()?
        .get("properties")?
        .as_object()?
        .get("name")?
        .as_str()?
        .rsplit_once(":")?
        .1
        .parse()
        .ok()
}
