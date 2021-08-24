use indoc::indoc;

/// Basic Test model containing a single json field.
pub fn json() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            json Json
        }"
    };

    schema.to_owned()
}

/// Basic Test model containing a single optional json field.
pub fn json_opt() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            json Json?
        }"
    };

    schema.to_owned()
}

/// Basic Test model containing a single optional json field.
pub fn json_default() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            json Json @default("null")
        }"#
    };

    schema.to_owned()
}
