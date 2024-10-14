use indoc::indoc;

/// Basic Test model containing a single geometry field.
pub fn geom() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            geom Geometry
        }"
    };

    schema.to_owned()
}
