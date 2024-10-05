use serde_json::{Map, Value};

pub(crate) fn get_geometry_crs(geojson: &Map<String, Value>) -> Option<&str> {
    geojson
        .get("crs")?
        .as_object()?
        .get("properties")?
        .as_object()?
        .get("name")?
        .as_str()
}

pub(crate) fn trim_redundent_crs(geojson: &mut Map<String, Value>) {
    let crs = get_geometry_crs(geojson);
    if matches!(crs, Some("EPSG:4326" | "EPSG:0")) {
        geojson.remove("crs");
    };
}
