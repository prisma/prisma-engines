use indoc::indoc;

/// Basic Test model containing a single geometry field.
pub fn geometry() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            geometry GeoJson
        }"
    };

    schema.to_owned()
}

/// Basic Test model containing a single optional geometry field.
pub fn geometry_opt() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            geometry GeoJson?
        }"
    };

    schema.to_owned()
}
