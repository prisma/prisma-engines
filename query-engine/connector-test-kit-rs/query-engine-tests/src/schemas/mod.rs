use indoc::indoc;

/// Most basic datamodel containing only a model with ID
/// for the most rudimentary testing.
pub fn basic() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            field String?
        }"
    };

    schema.to_owned()
}
