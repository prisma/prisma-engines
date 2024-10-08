#[allow(dead_code)]
pub(crate) fn get_geometry_crs(geojson: &serde_json::Map<String, serde_json::Value>) -> Option<&str> {
    geojson
        .get("crs")?
        .as_object()?
        .get("properties")?
        .as_object()?
        .get("name")?
        .as_str()
}

#[allow(dead_code)]
pub fn get_geometry_srid(geom: &geojson::Geometry) -> Option<i32> {
    geom.foreign_members
        .as_ref()
        .and_then(get_geometry_crs)?
        .rsplit_once(":")?
        .1
        .parse()
        .ok()
}
